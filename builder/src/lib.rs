use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, Ident, LitStr, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    render(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn render(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let DeriveInput {
        vis,
        ident,
        generics,
        data:
            Data::Struct(DataStruct {
                fields: Fields::Named(FieldsNamed { named, .. }),
                ..
            }),
        ..
    } = input
    else {
        return Err(syn::Error::new(
            input.span(),
            "expected `builder(each = \"...\")`",
        ));
    };

    let builder_name = format_ident!("{}Builder", ident);
    let builder_default_fields = named
        .iter()
        .map(builder_default_field)
        .collect::<syn::Result<Vec<_>>>()?;
    let builder_fields = named
        .iter()
        .map(builder_field)
        .collect::<syn::Result<Vec<_>>>()?;
    let builder_field_setters = named
        .iter()
        .map(builder_field_setter)
        .collect::<syn::Result<Vec<_>>>()?;
    let build_fields = named
        .iter()
        .map(build_field)
        .collect::<syn::Result<Vec<_>>>()?;

    let expanded = quote! {
        impl #generics #ident #generics {
            #vis fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_default_fields)*
                }
            }
        }

        #vis struct #builder_name #generics {
            #(#builder_fields)*
        }

        impl #builder_name {
            #(#builder_field_setters)*

            #vis fn build(&mut self) -> std::result::Result<#ident, std::boxed::Box<dyn std::error::Error>> {
                Ok(#ident {
                    #(#build_fields)*
                })
            }
        }

    };

    Ok(expanded)
}

fn builder_default_field(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    match FieldType::from(field)? {
        FieldType::Required { ident, .. } => Ok(quote! { #ident: std::option::Option::None, }),
        FieldType::Optional { ident, .. } => Ok(quote! { #ident: std::option::Option::None, }),
        FieldType::Each { ident, .. } => Ok(quote! { #ident: vec![], }),
    }
}

fn builder_field(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    match FieldType::from(field)? {
        FieldType::Optional { ident, ty, .. } => Ok(quote! {#ident: std::option::Option<#ty>,}),
        FieldType::Required { ident, ty, .. } => Ok(quote! {#ident: std::option::Option<#ty>,}),
        FieldType::Each { ident, ty, .. } => Ok(quote! {#ident: Vec<#ty>,}),
    }
}

fn builder_field_setter(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    match FieldType::from(field)? {
        FieldType::Required { ident, ty } => Ok(quote! {
            pub fn #ident(&mut self, #ident:#ty)->&mut Self {
                self.#ident = std::option::Option::Some(#ident);
                self
            }
        }),
        FieldType::Optional { ident, ty } => Ok(quote! {
            pub fn #ident(&mut self, #ident:#ty)->&mut Self {
                self.#ident = std::option::Option::Some(#ident);
                self
            }
        }),
        FieldType::Each {
            ident, setter, ty, ..
        } => Ok(quote! {
            pub fn #setter(&mut self, #setter:#ty)->&mut Self {
                self.#ident.push(#setter);
                self
            }
        }),
    }
}

fn build_field(field: &Field) -> syn::Result<proc_macro2::TokenStream> {
    match FieldType::from(field)? {
        FieldType::Required { ident, .. } => {
            Ok(quote! {#ident:self.#ident.clone().ok_or("not initialize")?,})
        }
        FieldType::Optional { ident, .. } => Ok(quote! {#ident:self.#ident.clone(),}),
        FieldType::Each { ident, .. } => Ok(quote! {#ident:self.#ident.clone(),}),
    }
}

fn parse_attr(attr: &Attribute) -> std::option::Option<syn::Result<String>> {
    if !attr.path().is_ident("builder") {
        return std::option::Option::None;
    };

    let mut name = String::new();
    let res = attr
        .parse_nested_meta(|meta| {
            if meta.path.is_ident("each") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                name = s.value();
                Ok(())
            } else {
                Err(syn::Error::new_spanned(
                    attr.meta.clone(),
                    "expected `builder(each = \"...\")`",
                ))
            }
        })
        .map(|_| name);
    std::option::Option::Some(res)
}

enum FieldType<'a> {
    Required {
        ident: std::option::Option<&'a Ident>,
        ty: &'a Type,
    },
    Optional {
        ident: std::option::Option<&'a Ident>,
        ty: &'a Type,
    },
    Each {
        ident: std::option::Option<&'a Ident>,
        ty: &'a Type,
        setter: Ident,
    },
}

impl<'a> FieldType<'a> {
    fn from(
        Field {
            ident, ty, attrs, ..
        }: &'a Field,
    ) -> syn::Result<Self> {
        match attrs.iter().find_map(parse_attr) {
            std::option::Option::Some(s) => {
                // Each: Vec<_>
                let Ok(name) = s else {
                    return Err(s.unwrap_err());
                };
                let (ty, id) = get_inner_type(ty).expect("incorrect attributes1");
                if id != "Vec" {
                    return Err(syn::Error::new(
                        ty.span(),
                        "shold be Vec for field with each",
                    ));
                };
                Ok(Self::Each {
                    ident: ident.as_ref(),
                    ty,
                    setter: format_ident!("{}", name),
                })
            }
            std::option::Option::None => {
                // Required: bare or Vec<_> or Optional: Option<_>
                if let std::option::Option::Some((ty_in, id)) = get_inner_type(ty) {
                    if id == "Option" {
                        Ok(Self::Optional {
                            ident: ident.as_ref(),
                            ty: ty_in,
                        })
                    } else {
                        Ok(Self::Required {
                            ident: ident.as_ref(),
                            ty,
                        })
                    }
                } else {
                    Ok(Self::Required {
                        ident: ident.as_ref(),
                        ty,
                    })
                }
            }
        }
    }
}

fn get_inner_type(ty: &Type) -> std::option::Option<(&Type, &Ident)> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let std::option::Option::Some(seg) = path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let std::option::Option::Some(syn::GenericArgument::Type(inner_ty)) =
                    args.args.first()
                {
                    return std::option::Option::Some((inner_ty, &seg.ident));
                }
            }
        }
    }
    std::option::Option::None
}
