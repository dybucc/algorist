use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, Expr, ExprCall, ExprField, ExprLit, ExprStruct, Field,
    GenericArgument, GenericParam, Generics, Ident, ImplItem, ImplItemFn, Index, ItemFn, ItemImpl,
    Lit, Path, PathArguments, Result as SynResult, Token, TraitBound, Type, TypeParamBound,
    TypeTuple, Visibility, WhereClause, WherePredicate, braced,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::Brace,
};

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
        use std::{
            any::{Any, TypeId},
            collections::HashMap,
        };

        struct SampleStruct(HashMap<TypeId, Vec<Box<dyn Any>>>);

        impl SampleStruct {
            fn sample_impl<T1, T2>(fields: (T1, T2)) -> Self
            where
                for<'a> T1: 'a,
                for<'a> T2: 'a,
            {
                let mut input: HashMap<_, Vec<Box<dyn Any>>> = HashMap::new();

                input.insert(fields.0.type_id(), vec![Box::new(fields.0)]);
                input.insert(fields.1.type_id(), vec![Box::new(fields.1)]);

                Self(input)
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

    (1..=16).for_each(|ident_state| {
        let (mut generics_output, mut where_output, mut params_output, mut block_output) = (
            Punctuated::<GenericParam, Token![,]>::new(),
            Punctuated::<WherePredicate, Token![,]>::new(),
            Punctuated::<Type, Token![,]>::new(),
            Punctuated::<ExprCall, Token![,]>::new(),
        );

        (1..=ident_state).for_each(|ident_state| {
            let ident = Ident::new(&format!("T{ident_state}"), Span::call_site());

            generics_output.push(parse_quote! { #ident });
            where_output.push(parse_quote! { for<'a> #ident: 'a });
            params_output.push(parse_quote! { #ident });

            let ident = Index {
                index: ident_state - 1,
                span: Span::call_site(),
            };
            let field_access: ExprField = parse_quote! { fields.#ident };

            block_output.push(parse_quote! { Box::new(#field_access) });
        });

        impl_block.items.push(ImplItem::Fn(ImplItemFn {
            attrs: Default::default(),
            vis: Visibility::Public(Default::default()),
            defaultness: None,
            sig: {
                let ident = Ident::new(&format!("with_{ident_state}"), Span::call_site());

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
            block: parse_quote! { { Self(vec![#block_output]) } },
        }));
    });

    TokenStream::from(quote! { #impl_block })
}

#[proc_macro_attribute]
pub fn add(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    TokenStream::from({
        let output = ItemFn {
            attrs: input.attrs,
            vis: input.vis,
            sig: {
                let (ident, inputs, output) = (input.sig.ident, input.sig.inputs, input.sig.output);
                let generics = input.sig.generics.params;
                let mut where_clause =
                    input.sig.generics.where_clause.expect(
                        "This attribute is designed for functions with `where` clauses only.",
                    );

                for bound in &mut where_clause.predicates {
                    if let WherePredicate::Type(pred) = bound {
                        let TypeParamBound::Trait(TraitBound {
                            path:
                                Path {
                                    segments: fields_trait,
                                    ..
                                },
                            ..
                        }) = pred.bounds.last().expect(
                            "There should be at least one trait bound in the function item tagged \
                            with this attribute.",
                        )
                        else {
                            panic!(
                                "The attribute should only be called on functions that use \
                                `Fields` as their last trait bound."
                            );
                        };
                        let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) = &fields_trait
                            .last()
                            .expect(
                                "There should at least be one trait bound for the `Fields` trait.",
                            )
                            .arguments
                        else {
                            panic!(
                                "The attribute should only be applied to functions that use \
                                `Fields` as the last trait bound."
                            );
                        };
                        let (
                            GenericArgument::Type(inner_ty),
                            GenericArgument::Const(Expr::Lit(ExprLit {
                                lit: Lit::Int(inner_n),
                                ..
                            })),
                        ) = (
                            args.first().expect(
                                "The `Field` trait bound includes as its first argument the type \
                                of the required fields.",
                            ),
                            args.last().expect(
                                "The `Field` trait bound includes as its second argument the \
                                amount fields it requires.",
                            ),
                        )
                        else {
                            panic!(
                                "The `Field` trait bound includes as its first argument the type \
                                of the required fields, and as its second argument the amount \
                                fields it requires."
                            );
                        };
                        let (ty, n) = (
                            inner_ty.clone(),
                            inner_n.base10_parse::<usize>().expect(
                                "The second argument to the `Fields` trait bound is always an \
                                integer.",
                            ),
                        );

                        (0..n).for_each(|idx| {
                            let n: ExprLit = parse_quote! { #idx };
                            pred.bounds.push(parse_quote! { Field<#ty, #n> });
                        });
                    }
                }

                parse_quote! { fn #ident <#generics> ( #inputs ) #where_clause #output }
            },
            block: input.block,
        };

        quote! { #output }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn it_works() {
        let input: ItemFn = parse_quote! {
            fn planar_graph<T>(graph: &mut T)
            where
                T: GraphBackend + Fields<String, 2>,
                T::Vertex: Fields<u32, 3>,
            {
            }
        };

        eprintln!("{:#?}", {
            let output = ItemFn {
                attrs: input.attrs,
                vis: input.vis,
                sig: {
                    let (ident, inputs, output) =
                        (input.sig.ident, input.sig.inputs, input.sig.output);
                    let generics = input.sig.generics.params;
                    let mut where_clause = input.sig.generics.where_clause.expect(
                        "This attribute is designed for functions with `where` clauses only.",
                    );

                    for bound in &mut where_clause.predicates {
                        if let WherePredicate::Type(pred) = bound {
                            let TypeParamBound::Trait(TraitBound {
                            path:
                                Path {
                                    segments: fields_trait,
                                    ..
                                },
                            ..
                        }) = pred.bounds.last().expect(
                            "There should be at least one trait bound in the function item tagged \
                            with this attribute.",
                        )
                        else {
                            panic!(
                                "The attribute should only be called on functions that use \
                                `Fields` as their last trait bound."
                            );
                        };
                            let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) = &fields_trait
                            .last()
                            .expect(
                                "There should at least be one trait bound for the `Fields` trait.",
                            )
                            .arguments
                        else {
                            panic!(
                                "The attribute should only be applied to functions that use \
                                `Fields` as the last trait bound."
                            );
                        };
                            let (
                            GenericArgument::Type(inner_ty),
                            GenericArgument::Const(Expr::Lit(ExprLit {
                                lit: Lit::Int(inner_n),
                                ..
                            })),
                        ) = (
                            args.first().expect(
                                "The `Field` trait bound includes as its first argument the type \
                                of the required fields.",
                            ),
                            args.last().expect(
                                "The `Field` trait bound includes as its second argument the \
                                amount fields it requires.",
                            ),
                        )
                        else {
                            panic!(
                                "The `Field` trait bound includes as its first argument the type \
                                of the required fields, and as its second argument the amount \
                                fields it requires."
                            );
                        };
                            let (ty, n) = (
                                inner_ty.clone(),
                                inner_n.base10_parse::<usize>().expect(
                                    "The second argument to the `Fields` trait bound is always an \
                                integer.",
                                ),
                            );

                            (0..n).for_each(|idx| {
                                let n: ExprLit = parse_quote! { #idx };
                                pred.bounds.push(parse_quote! { Field<#ty, #n> });
                            });
                        }
                    }

                    parse_quote! { fn #ident <#generics> ( #inputs ) #where_clause #output }
                },
                block: input.block,
            };

            quote! { #output }
        });
    }
}
