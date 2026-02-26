use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DataStruct, DeriveInput, Expr, ExprLit, Field,
    GenericParam, Generics, Lit, Meta, MetaNameValue,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    render(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn render(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics,
        data,
    } = input;

    let Data::Struct(DataStruct { fields, .. }) = data else {
        return Err(syn::Error::new_spanned(&ident, "must be struct"));
    };

    let generics = add_trait_bound(&generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fmt_fields = fields.iter().map(fmt_fields).collect::<Vec<_>>();

    let output = quote! {
        impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>)->std::result::Result<(),std::fmt::Error> {
                f.debug_struct(stringify!(#ident))
                    #(#fmt_fields)*
                    .finish()
            }
        }
    };
    Ok(output)
}

fn fmt_fields(Field { ident, attrs, .. }: &Field) -> proc_macro2::TokenStream {
    let res = attrs.iter().find_map(parse_attr);
    if let Some(fmt) = res {
        quote! {
            .field(stringify!(#ident), &format_args!(#fmt, &self.#ident))
        }
    } else {
        quote! {
            .field(stringify!(#ident), &self.#ident)
        }
    }
}

fn parse_attr(attr: &Attribute) -> std::option::Option<String> {
    if !attr.path().is_ident("debug") {
        return std::option::Option::None;
    }

    let Meta::NameValue(MetaNameValue {
        value: Expr::Lit(ExprLit {
            lit: Lit::Str(ref lit_str),
            ..
        }),
        ..
    }) = attr.meta
    else {
        return std::option::Option::None;
    };
    Some(lit_str.value())
}

fn add_trait_bound(generics: &Generics) -> Generics {
    let mut new_generics = generics.clone();
    for param in &mut new_generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        };
    }
    new_generics
}
