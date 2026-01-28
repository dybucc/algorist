use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    ExprCall, ExprField, ExprStruct, Field, GenericParam, Generics, Ident, ImplItem, ImplItemFn,
    Index, ItemImpl, Path, Result as SynResult, Token, Type, TypeTuple, Visibility, WhereClause,
    WherePredicate, braced,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::Brace,
};

use std::any::Any;

struct Primitive {
    _struct_token: Token![struct],
    type_name: Ident,
    _brace_token: Brace,
    existing_fields: Punctuated<Field, Token![,]>,
}

impl Primitive {
    fn add_fields(&mut self, extra: Tweaks) {
        for field in extra.additional_fields.into_iter() {
            self.existing_fields.push(field);
        }
    }

    #[must_use = "The primitive type is gone now; Only tokens remain..."]
    fn tokenize(self) -> TokenStream2 {
        let type_name = self.type_name;
        let existing_fields = self.existing_fields;

        quote! {
            struct #type_name {
                #existing_fields
            }
        }
    }
}

impl Parse for Primitive {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;

        Ok(Self {
            _struct_token: input.parse()?,
            type_name: input.parse()?,
            _brace_token: braced!(content in input),
            existing_fields: content.parse_terminated(Field::parse_named, Token![,])?,
        })
    }
}

struct Tweaks {
    additional_fields: Punctuated<Field, Token![,]>,
}

impl Parse for Tweaks {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(Self {
            additional_fields: input.parse_terminated(Field::parse_named, Token![,])?,
        })
    }
}

#[proc_macro_attribute]
pub fn add_fields(changes: TokenStream, subject: TokenStream) -> TokenStream {
    let (changes, mut subject) = (
        parse_macro_input!(changes as Tweaks),
        parse_macro_input!(subject as Primitive),
    );

    subject.add_fields(changes);

    TokenStream::from(subject.tokenize())
}

struct SeqPrimitive(Punctuated<ExprStruct, Token![,]>);

impl SeqPrimitive {
    fn tokenize(self) -> TokenStream2 {
        let (mut arc_fields, mut vertex_fields, mut graph_fields) = (None, None, None);

        for ExprStruct {
            path: Path {
                segments: ident, ..
            },
            fields,
            ..
        } in self.0
        {
            match ident
                .first()
                .expect(
                    "The identifier of the graph primitive should always appear in the macro \
                    invocation.",
                )
                .ident
                .to_string()
                .as_str()
            {
                "Graph" => graph_fields = Some(fields),
                "Vertex" => vertex_fields = Some(fields),
                "Arc" => arc_fields = Some(fields),
                _ => (),
            }
        }

        quote! {
            struct Arc<'a> {
                tip: &'a Vertex,
                #arc_fields
            }

            struct Vertex<'a> {
                arcs: Vec<&'a Arc<'a>>,
                #vertex_fields
            }

            struct Graph {
                vertices: Vec<Vertex<'a>>,
                arcs: Vec<Arc>,
                n: usize,
                m: usize,
                id: usize,
                #graph_fields
            }
        }
    }
}

impl Parse for SeqPrimitive {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(Self(input.parse_terminated(ExprStruct::parse, Token![,])?))
    }
}

#[proc_macro]
pub fn declare(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SeqPrimitive);

    TokenStream::from(input.tokenize())
}

#[proc_macro_derive(TupleConstr)]
pub fn gen_tuple_constructors(_: TokenStream) -> TokenStream {
    // Model:
    #[expect(unused)]
    {
        struct SampleStruct(Vec<Box<dyn Any>>);

        impl SampleStruct {
            fn sample_impl<T1, T2>(fields: (T1, T2)) -> Self
            where
                for<'a> T1: 'a + Any,
                for<'a> T2: 'a + Any,
            {
                Self(vec![Box::new(fields.0), Box::new(fields.1)])
            }
        }
    }

    let mut impl_block = ItemImpl {
        attrs: Default::default(),
        defaultness: None,
        unsafety: None,
        impl_token: Default::default(),
        generics: Default::default(),
        trait_: None,
        self_ty: Box::new(parse_quote! { FieldBuilder }),
        brace_token: Default::default(),
        items: Vec::with_capacity(1000),
    };

    (1..=1000).for_each(|ident_state| {
        impl_block.items.push(ImplItem::Fn(ImplItemFn {
            attrs: Default::default(),
            vis: Visibility::Public(Default::default()),
            defaultness: None,
            sig: {
                let ident = Ident::new(&format!("with_{ident_state}"), Span::call_site());
                let (mut generics_output, mut where_output, mut params_output) = (
                    Punctuated::<GenericParam, Token![,]>::new(),
                    Punctuated::<WherePredicate, Token![,]>::new(),
                    Punctuated::<Type, Token![,]>::new(),
                );

                (1..=ident_state).for_each(|ident_state| {
                    let ident = Ident::new(&format!("T{ident_state}"), Span::call_site());

                    generics_output.push(parse_quote! { #ident });
                    where_output.push(parse_quote! { for<'a> #ident: 'a + Any });
                    params_output.push(parse_quote! { #ident });
                });

                let (generics, params, where_clause): (_, _, WhereClause) = (
                    Generics {
                        lt_token: Default::default(),
                        params: generics_output,
                        gt_token: Default::default(),
                        where_clause: None,
                    },
                    TypeTuple {
                        paren_token: Default::default(),
                        elems: params_output,
                    },
                    parse_quote! { where #where_output },
                );

                parse_quote! { fn #ident #generics (fields: #params) -> Self #where_clause }
            },
            block: {
                let mut output = Punctuated::<ExprCall, Token![,]>::new();

                (1..=ident_state).for_each(|ident_state| {
                    let ident = Index {
                        index: ident_state - 1,
                        span: Span::call_site(),
                    };
                    let field_access: ExprField = parse_quote! { fields.#ident };

                    output.push(parse_quote! { Box::new(#field_access) });
                });

                parse_quote! { { Self(vec![#output]) } }
            },
        }));
    });

    TokenStream::from(quote! { #impl_block })
}

#[cfg(test)]
mod tests {
    use syn::{
        ExprCall, ExprField, Generics, Index, TypeTuple, WhereClause, WherePredicate, parse_quote,
    };

    use super::*;

    #[test]
    fn it_works() {
        let mut impl_block = ItemImpl {
            attrs: Default::default(),
            defaultness: None,
            unsafety: None,
            impl_token: Default::default(),
            generics: Default::default(),
            trait_: None,
            self_ty: Box::new(parse_quote! { FieldBuilder }),
            brace_token: Default::default(),
            items: Vec::with_capacity(1000),
        };

        (1..=1000).for_each(|ident_state| {
            impl_block.items.push(ImplItem::Fn(ImplItemFn {
                attrs: Default::default(),
                vis: Visibility::Public(Default::default()),
                defaultness: None,
                sig: {
                    let ident = Ident::new(&format!("with_{ident_state}"), Span::call_site());
                    let (mut generics_output, mut where_output, mut params_output) = (
                        Punctuated::<GenericParam, Token![,]>::new(),
                        Punctuated::<WherePredicate, Token![,]>::new(),
                        Punctuated::<Type, Token![,]>::new(),
                    );

                    (1..=ident_state).for_each(|ident_state| {
                        let ident = Ident::new(&format!("T{ident_state}"), Span::call_site());

                        generics_output.push(parse_quote! { #ident });
                        where_output.push(parse_quote! { for<'a> #ident: 'a + FieldElem });
                        params_output.push(parse_quote! { #ident });
                    });

                    let (generics, params, where_clause): (_, _, WhereClause) = (
                        Generics {
                            lt_token: Default::default(),
                            params: generics_output,
                            gt_token: Default::default(),
                            where_clause: None,
                        },
                        TypeTuple {
                            paren_token: Default::default(),
                            elems: params_output,
                        },
                        parse_quote! { where #where_output },
                    );

                    parse_quote! { fn #ident #generics (fields: #params) -> Self #where_clause }
                },
                block: {
                    let mut output = Punctuated::<ExprCall, Token![,]>::new();

                    (1..=ident_state).for_each(|ident_state| {
                        let ident = Index {
                            index: ident_state - 1,
                            span: Span::call_site(),
                        };
                        let field_access: ExprField = parse_quote! { fields.#ident };

                        output.push(parse_quote! { Box::new(#field_access) });
                    });

                    parse_quote! { { Self(vec![#output]) } }
                },
            }));
        });

        eprintln!("{}", quote! { #impl_block });
    }
}
