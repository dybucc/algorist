#![expect(dead_code, reason = "The crate is a WIP.")]

use crate::api::{GraphBackend, IndexerExt, VertexEntryExt};

mod api;
mod fields;
mod private {
    pub(crate) trait Sealed {}
}

const ARC_ALLOCS: usize = 102;
const EXTRA_N: usize = 4;

#[derive(Debug)]
struct Arc<'a> {
    tip: &'a Vertex<'a>,
}

#[derive(Debug)]
struct Vertex<'a> {
    arcs: Vec<&'a Arc<'a>>,
}

#[derive(Debug)]
struct Graph<'a> {
    vertices: Vec<Vertex<'a>>,
    arcs: Vec<Arc<'a>>,
    n: usize,
    m: usize,
    // TODO: see whether this deserves its own API; The generative routines
    // don't seem to mind it much, but something could be implemented around
    // existing functionality from the original GraphBase. See GB_GRAPH, Sec.
    // 26, 27.
    id: Option<usize>,
}

struct GraphError(GraphErrorReason);

enum GraphErrorReason {
    NoVerticesInGraph,
}

enum VertexEntry<'a> {
    Shared(*const Graph<'a>, *const Vertex<'a>),
    Mutable(*mut Graph<'a>, *mut Vertex<'a>),
}

impl IndexerExt for usize {
    fn get(&self) -> Self {
        *self
    }
}

impl<'a> VertexEntryExt for Option<VertexEntry<'a>> {
    fn and_insert_arc(&mut self, f: impl FnMut(&mut Self, Self)) {
        todo!()
    }
}

impl<'a> GraphBackend for Graph<'a> {
    type Vertex = Vertex<'a>;
    type Arc = Arc<'a>;

    type VertexEntry = VertexEntry<'a>;

    type Indexer = usize;
    type Error = GraphError;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n + EXTRA_N);
                output.resize_with(n, || Vertex { arcs: Vec::new() });

                output
            },
            arcs: Vec::new(),
            n,
            m: 0,
            id: None,
        }
    }

    fn n(&self) -> usize {
        self.n
    }
    fn m(&self) -> usize {
        self.m
    }

    fn get(&self, idx: Self::Indexer) -> Option<Self::VertexEntry> {
        todo!()
    }
    fn get_mut(&mut self, idx: Self::Indexer) -> Option<Self::VertexEntry> {
        todo!()
    }

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // let mut graph = Graph::new(10);

        // graph.get_mut(3).expect("test").and_insert_arc(2);

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
