#![allow(dead_code, reason = "The crate is a WIP.")]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{
    Field, FieldMutability, Ident, Path, PathArguments, PathSegment, Token, Type, TypePath,
    Visibility, braced, parse::Parse, parse_macro_input, punctuated::Punctuated, token::Brace,
};

struct Primitive {
    struct_token: Token![struct],
    type_name: Ident,
    brace_token: Brace,
    existing_fields: Punctuated<Field, Token![,]>,
}

impl Parse for Primitive {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            struct_token: input.parse()?,
            type_name: input.parse()?,
            brace_token: braced!(content in input),
            existing_fields: content.parse_terminated(Field::parse_named, Token![,])?,
        })
    }
}

struct Tweaks {
    additional_fields: Punctuated<Field, Token![,]>,
}

impl Parse for Tweaks {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        dbg!(&input);

        Ok(Self {
            additional_fields: {
                let mut output = Punctuated::new();
                output.push(Field {
                    attrs: Vec::new(),
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: None,
                    colon_token: None,
                    ty: Type::Path(TypePath {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: {
                                let mut output = Punctuated::new();
                                output.push(PathSegment {
                                    ident: Ident::new("", Span::call_site()),
                                    arguments: PathArguments::None,
                                });

                                output
                            },
                        },
                    }),
                });

                output
            },
        })
    }
}

#[proc_macro_attribute]
pub fn add_field(changes: TokenStream, subject: TokenStream) -> TokenStream {
    let (changes, subject) = (
        parse_macro_input!(changes as Tweaks),
        parse_macro_input!(subject as Primitive),
    );

    TokenStream::from(TokenStream2::default())
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use syn::{
        FieldMutability, Path, PathArguments, PathSegment, Type, TypePath, Visibility, parse_quote,
    };

    use super::*;

    #[test]
    fn it_works() {
        let mut preproc_input: Primitive = parse_quote! {
            struct Graph {
                vertices: AdjacencyList,
                m: usize,
                n: usize,
                id: usize
            }
        };

        preproc_input.existing_fields.push(Field {
            attrs: Vec::new(),
            vis: Visibility::Inherited,
            mutability: FieldMutability::None,
            ident: Some(Ident::new("name", Span::call_site())),
            colon_token: Some(Token![:](Span::call_site())),
            ty: Type::Path(TypePath {
                qself: None,
                path: Path {
                    leading_colon: None,
                    segments: {
                        let mut output = Punctuated::new();
                        output.push(PathSegment {
                            ident: Ident::new("std", Span::call_site()),
                            arguments: PathArguments::None,
                        });
                        output.push(PathSegment {
                            ident: Ident::new("string", Span::call_site()),
                            arguments: PathArguments::None,
                        });
                        output.push(PathSegment {
                            ident: Ident::new("String", Span::call_site()),
                            arguments: PathArguments::None,
                        });

                        output
                    },
                },
            }),
        });

        let proc_details: Tweaks = parse_quote! {
            #[add_field {
                name: String,
            }]
        };
    }
}
