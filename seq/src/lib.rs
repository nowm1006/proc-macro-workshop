#![allow(dead_code, unused_variables)]
mod proc_impl;
use proc_impl::Seq;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Seq);
    quote!(#input).into()
    // eprintln!("{:?}", &output);
    // output
}
