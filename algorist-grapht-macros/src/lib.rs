use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, Block, DeriveInput, ExprStruct, Field, FnArg, GenericArgument,
    Ident, ImplItem, ImplItemFn, ItemImpl, Pat, PatIdent, PatType, Path, PathArguments,
    PathSegment, Result as SynResult, ReturnType, Signature, Token, TraitBound, TraitBoundModifier,
    Type, TypeParamBound, TypePath, TypeTraitObject, TypeTuple, Visibility, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Brace, PathSep},
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

#[non_exhaustive]
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
pub fn gen_tuple_constructors(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let ident = derive_input.ident;
    let mut functions = ItemImpl {
        attrs: Default::default(),
        defaultness: None,
        unsafety: None,
        impl_token: Default::default(),
        generics: Default::default(),
        trait_: None,
        self_ty: Box::new(Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments: {
                    let mut output = Punctuated::new();
                    output.push(PathSegment {
                        ident: Ident::new("FieldBuilder", Span::call_site()),
                        arguments: PathArguments::None,
                    });

                    output
                },
            },
        })),
        brace_token: Default::default(),
        items: Vec::new(),
    };

    let mut ident_state = 1_usize;
    let dyn_object = {
        let mut output = Punctuated::<PathSegment, PathSep>::new();
        output.push(PathSegment {
            ident: Ident::new("Box", Span::call_site()),
            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: Default::default(),
                args: {
                    let mut output = Punctuated::new();
                    output.push(GenericArgument::Type(Type::TraitObject(TypeTraitObject {
                        dyn_token: Default::default(),
                        bounds: {
                            let mut output = Punctuated::new();
                            output.push(TypeParamBound::Trait(TraitBound {
                                paren_token: Default::default(),
                                modifier: TraitBoundModifier::None,
                                lifetimes: None,
                                path: Path {
                                    leading_colon: None,
                                    segments: {
                                        let mut output = Punctuated::new();
                                        output.push(PathSegment {
                                            ident: Ident::new("FieldElem", Span::call_site()),
                                            arguments: PathArguments::None,
                                        });

                                        output
                                    },
                                },
                            }));

                            output
                        },
                    })));

                    output
                },
                gt_token: Default::default(),
            }),
        });

        output
    };
    loop {
        if ident_state == 1001 {
            break;
        }

        functions.items.push(ImplItem::Fn(ImplItemFn {
            attrs: Default::default(),
            vis: Visibility::Public(Default::default()),
            defaultness: None,
            sig: Signature {
                ident: Ident::new(&format!("with_{ident_state}"), Span::call_site()),
                inputs: {
                    let mut output = Punctuated::new();

                    output.push_value(FnArg::Typed(PatType {
                        attrs: Default::default(),
                        pat: Box::new(Pat::Ident(PatIdent {
                            attrs: Default::default(),
                            by_ref: None,
                            mutability: None,
                            ident: Ident::new("fields", Span::call_site()),
                            subpat: None,
                        })),
                        colon_token: Default::default(),
                        ty: Box::new(Type::Tuple(TypeTuple {
                            paren_token: Default::default(),
                            elems: {
                                let (mut output, mut count) = (Punctuated::new(), ident_state);

                                while count != 0 {
                                    output.push(Type::Path(TypePath {
                                        qself: None,
                                        path: Path {
                                            leading_colon: None,
                                            segments: dyn_object.clone(),
                                        },
                                    }));
                                    count -= 1;
                                }

                                output
                            },
                        })),
                    }));

                    output
                },
                output: ReturnType::Type(
                    Default::default(),
                    Box::new(Type::Path(TypePath {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: {
                                let mut output = Punctuated::new();
                                output.push(PathSegment {
                                    ident: Ident::new("Self", Span::call_site()),
                                    arguments: PathArguments::None,
                                });

                                output
                            },
                        },
                    })),
                ),
                constness: None,
                asyncness: None,
                unsafety: None,
                abi: None,
                variadic: None,
                fn_token: Default::default(),
                generics: Default::default(),
                paren_token: Default::default(),
            },
            block: Block {
                brace_token: Default::default(),
                stmts: {
                    let mut output = Vec::new();

                    // Model:
                    // trait Sample {}
                    // struct SampleStruct(Vec<Box<dyn Sample>>);

                    // fn sample_impl(fields: (Box<dyn Sample>, Box<dyn Sample>)) -> SampleStruct {
                    //     SampleStruct(vec![fields.0, fields.1])
                    // }

                    output
                },
            },
        }));

        ident_state += 1;
    }

    TokenStream::from(quote! {
        #functions
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn it_works() {
        let preproc_input: SeqPrimitive = parse_quote! {
            Graph { name: String },
            Vertex { name: String },
            Arc { name: String }
        };
        eprintln!(
            "{}",
            quote! {
                Graph { name: String },
                Vertex { name: String },
                Arc { name: String }
            }
        );
        eprintln!();

        let output = preproc_input.tokenize();
        eprintln!("{}", output);
    }
}
