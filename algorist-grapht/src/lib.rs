#![feature(
    associated_type_defaults,
    ascii_char,
    min_specialization,
    allocator_api
)]
#![expect(dead_code, reason = "The crate is a WIP.")]

pub mod api;
pub mod backend;
pub mod fields;
mod private {
    pub(crate) trait Sealed {}
}

#[cfg(test)]
mod tests {
    use crate::{api::GraphBackend, backend::Graph};

    #[test]
    fn it_works() {
        let graph = Graph::new(10);

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
