use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, Fields};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as syn::ItemStruct);

    render(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn render(input: &syn::ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let tys = input
        .fields
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();
    let attr_bits_check = input.fields.iter().map(|field| {
        let bits = field.attrs.iter().find_map(|attr| {
            if let syn::Meta::NameValue(ref name_value) = attr.meta {
                if name_value.path.is_ident("bits") {
                    let value = &name_value.value;
                    let span = name_value.value.span();
                    return Some((value, span));
                };
            };
            None
        });
        let ty = &field.ty;
        let field_bits = quote! { #ty::BITS };
        if bits.is_some() {
            let (bits, span) = bits.unwrap();
            Some(quote_spanned! {span =>
                const _:[();#bits] = [();#field_bits];
            })
        } else {
            None
        }
    });
    let total_bytes = quote! { (0 #(+ #tys::BITS)* )/8 };
    let total_bits = quote! { (0 #(+ #tys::BITS)* )};
    let accessors = accessors(&input.fields);
    Ok(quote! {
        #[repr(C)]
        #[derive(Default)]
        pub struct #ident {
            data: [u8; #total_bytes],
        }

        impl #ident {
            fn new()->Self {
                Default::default()
            }

            #( #accessors )*

        }

        bitfield::require_multiple_of_eight!( #total_bits );
        #( #attr_bits_check )*

    })
}

fn accessors<'a>(fields: &'a Fields) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    fields.iter().enumerate().filter_map(|(i, field)| {
        let ident = field.ident.as_ref()?;
        let ty = &field.ty;
        let offset = fields.iter().take(i).map(|field| {
            let ty = &field.ty;
            quote! { #ty::BITS }
        });
        let offset = quote! { 0 #(+ #offset )* };
        let bits = quote! { #ty::BITS };
        let field_type = quote! { <#ty as Specifier>::T };
        let getter_name = format_ident!("get_{}", &ident);
        let setter_name = format_ident!("set_{}", &ident);
        Some(quote! {
            fn #getter_name(&self) -> #field_type {
                let bits = #bits;
                let offset = #offset;
                let res = (offset..(offset+bits)).map(|i|{
                    let byte_idx = i / 8;
                    let bit_pos = 7 - (i % 8);
                    (self.data[byte_idx] >> bit_pos) & 1
                }).fold(0u64, |acc, e|{
                    (acc << 1) | e as u64
                });
                <#ty as Specifier>::convert_from_u64(res)
            }

            fn #setter_name(&mut self, value: #field_type) {
                let value = <#ty as Specifier>::convert_to_u64(value);
                let bits = #bits;
                let offset = #offset;
                (offset..(offset+bits)).for_each(|i|{
                    let byte_idx = i / 8;
                    let bit_pos = 7 - (i % 8);
                    let shift = offset + bits - 1 - i;
                    let v = (value >> shift) & 1;
                    self.data[byte_idx] &= !(1 << bit_pos);
                    self.data[byte_idx] |= (v << bit_pos) as u8;
                })
            }
        })
    })
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive_specifier(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let ident = &input.ident;
    let syn::Data::Enum(ref data_enum) = input.data else {
        return syn::Error::into_compile_error(syn::Error::new_spanned(
            input,
            "only enum is applicable",
        ))
        .into();
    };
    let n = data_enum.variants.len();
    if !n.is_power_of_two() {
        return syn::Error::into_compile_error(syn::Error::new(
            Span::call_site(),
            "BitfieldSpecifier expected a number of variants which is a power of 2",
        ))
        .into();
    }
    let bn = n.ilog2() as usize;
    let match_arms = data_enum.variants.iter().map(|variant| {
        // let discriminant = &variant.discriminant.as_ref().unwrap().1;
        let variant = &variant.ident;
        quote! {
            _ if #ident::#variant as u64 == value => #ident::#variant,
        }
    });
    let arm_sizes = data_enum.variants.iter().map(|variant| {
        let variant = &variant.ident;
        quote! { #ident::#variant as usize }
    });
    quote! {
        impl Specifier for #ident {
            const BITS: usize = #bn;
            type T = #ident;

            fn convert_from_u64(value: u64)-> Self::T {
                match value {
                    #( #match_arms )*
                    _ => panic!("unexpected value"),
                }
            }

            fn convert_to_u64(item: Self::T) -> u64 {
                item as u64
            }
        }

        const _:() = {
            let arms = [#( #arm_sizes, )*];
            let mut max = 0;
            let mut i = 0;
            while i < arms.len() {
                if arms[i] > max {
                    max = arms[i];
                }
                i += 1;
            }
            let bits = 1 << #bn;
            if max >= bits {
                panic!("bitfield size too large");
            };
        };
    }
    .into()
}
