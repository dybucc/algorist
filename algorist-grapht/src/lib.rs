#![feature(associated_type_defaults, try_reserve_kind, ascii_char)]
#![expect(dead_code, reason = "The crate is a WIP.")]

use std::ascii::Char;

mod api;
mod backend;
mod fields;
mod private {
    pub(crate) trait Sealed {}
}

#[inline]
fn parse_ascii_char(input: &[Char]) -> &str {
    input.as_str()
}

#[macro_export]
macro_rules! error {
    ($e:expr) => {
        $e.as_ascii()
            .expect("error messages don't contain non-ascii characters")
            .into()
    };
}

#[cfg(test)]
mod tests {
    #[expect(unused, reason = "WIP.")]
    use super::*;

    #[test]
    fn it_works() {
        // // TODO: implement a macro that lets me access each field more
        // // ergonomically inside of the function.
        // #[cfg_attr(not(doc), add)]
        // fn planar_graph<T>(g: &T)
        // where
        //     T: GraphBackend + Fields<String, 2>,
        //     T::Vertex: Fields<u32, 1>,
        // {
        //     <T as Field<String, 0>>::get(g);
        //     <T::Vertex as Field<u32, 0>>::get(
        //         <T as GraphBackend>::get(g, <T as GraphBackend>::Indexer { field: 0 }).unwrap(),
        //     );
        // }
    }
}
