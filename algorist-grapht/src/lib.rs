#![feature(allocator_api, try_with_capacity)]
#![expect(dead_code, reason = "The crate is a WIP.")]

pub mod api;
pub mod backend;
pub mod fields;
mod private {
    pub(crate) trait Sealed {}
}

#[cfg(test)]
mod tests {
    use crate::{
        api::{Command, GraphBackend, Selection, Insertion},
        backend::Graph,
    };

    #[test]
    fn it_works() -> <Graph as GraphBackend>::Result<()> {
        let mut graph = Graph::new(10)?;

        graph.cmd(Insertion::Arc(graph.iter_mut().select(0..1)));
        graph.cmd(Insertion::Arc(graph.iter_mut().select(0..=2)));
        graph.cmd(Insertion::Vertex);

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

        Ok(())
    }
}
