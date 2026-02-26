use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use quote::ToTokens;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{braced, LitInt, Result, Token};

#[derive(Debug)]
pub struct Seq {
    ident: Ident,
    start: usize,
    end: usize,
    tokens: TokenStream,
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![in]>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Token![..]>()?;
        let is_inclusive = input.peek(Token![=]);
        if is_inclusive {
            let _: Token![=] = input.parse()?;
        }
        let end =
            input.parse::<LitInt>()?.base10_parse::<usize>()? + if is_inclusive { 1 } else { 0 };
        let content: ParseBuffer<'_>;
        braced!(content in input);
        let tokens = content.parse::<TokenStream>()?;
        Ok(Self {
            ident,
            start,
            end,
            tokens,
        })
    }
}

impl ToTokens for Seq {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if contains_block(&self.tokens) {
            let stream = walk(&self.tokens, &self.ident, self.start, self.end);
            tokens.extend(stream);
        } else {
            (self.start..self.end).for_each(|n| {
                let stream = replace(&self.tokens, &self.ident, n);
                tokens.extend(stream);
            });
        }
    }
}

fn contains_block(input: &TokenStream) -> bool {
    let mut iter = input.clone().into_iter();
    while let Some(tree) = iter.next() {
        match tree {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some(TokenTree::Group(g)) = iter.next() {
                    if g.delimiter() == Delimiter::Parenthesis {
                        return true;
                    }
                }
            }
            TokenTree::Group(g) => {
                if contains_block(&g.stream()) {
                    return true;
                }
            }
            _ => (),
        }
    }
    false
}

fn walk(input: &TokenStream, ident: &Ident, start: usize, end: usize) -> TokenStream {
    let mut result = TokenStream::new();
    let mut iter = input.clone().into_iter().peekable();
    while let Some(tree) = iter.next() {
        'process: {
            match tree {
                TokenTree::Punct(p) if p.as_char() == '#' => {
                    if let Some(TokenTree::Group(ng)) = iter.peek() {
                        if ng.delimiter() == Delimiter::Parenthesis {
                            let Some(TokenTree::Group(g)) = iter.next() else {
                                panic!();
                            };
                            let new_stream = (start..end)
                                .map(|n| replace(&g.stream(), ident, n))
                                .collect::<TokenStream>();
                            result.extend(new_stream);
                            let _ = iter.next(); // consume *
                            break 'process;
                        }
                    }
                    result.extend(std::iter::once(p));
                }
                TokenTree::Group(g) => {
                    let new_stream = walk(&g.stream(), ident, start, end);
                    let mut group = Group::new(g.delimiter(), new_stream);
                    group.set_span(g.span());
                    result.extend(std::iter::once(group));
                }
                _ => result.extend(std::iter::once(tree)),
            }
        }
    }
    result
}

fn replace(input: &TokenStream, ident: &Ident, n: usize) -> TokenStream {
    let mut iter = input.clone().into_iter().peekable();
    let mut result = TokenStream::new();
    while let Some(tree) = iter.next() {
        'processing: {
            match tree {
                TokenTree::Ident(ref i) if i == ident => {
                    result.extend(std::iter::once(Literal::usize_unsuffixed(n)));
                }
                TokenTree::Ident(mut i) => {
                    if let Some(TokenTree::Punct(p)) = iter.peek() {
                        if p.as_char() == '~' {
                            // ident followed by ~
                            let _ = iter.next(); // consume ~
                            let Some(TokenTree::Ident(inext)) = iter.next() else {
                                panic!("~ must be followed by ident");
                            };
                            let new_ident = if &inext == ident {
                                Ident::new(&format!("{}{}", i, n), i.span())
                            } else {
                                Ident::new(&format!("{}{}", i, inext), i.span())
                            };
                            result.extend(std::iter::once(new_ident));
                            break 'processing;
                        }
                    }
                    // stand alone indent
                    if &i == ident {
                        i = Ident::new(&format!("{}", n), i.span());
                    }
                    result.extend(std::iter::once(i));
                }
                TokenTree::Group(g) => {
                    let new_stream = replace(&g.stream(), ident, n);
                    let mut group = Group::new(g.delimiter(), new_stream);
                    group.set_span(g.span());
                    result.extend(std::iter::once(group));
                }
                _ => result.extend(std::iter::once(tree)),
            };
        }
    }
    result
}
