use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Field, Ident, Result as SynResult, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
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

struct ListPrimitive {
    ident: Ident,
    brace_token: Brace,
    fields: Punctuated<Field, Token![,]>,
}

impl ListPrimitive {
    fn parse(input: ParseStream) -> SynResult<Self> {
        todo!();
    }
}

struct SeqPrimitive(Punctuated<ListPrimitive, Token![,]>);

impl Parse for SeqPrimitive {
    fn parse(input: ParseStream) -> SynResult<Self> {
        Ok(Self(
            input.parse_terminated(ListPrimitive::parse, Token![,])?,
        ))
    }
}

#[proc_macro]
pub fn graph(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SeqPrimitive);

    TokenStream::from(TokenStream2::default())
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn it_works() {
        let preproc_input: SeqPrimitive = parse_quote! {
            with
                Graph { name: String },
                Vertex { name: String },
                Arc { name: String }
        };
    }
}
