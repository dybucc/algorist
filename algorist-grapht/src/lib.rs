#![allow(dead_code, reason = "The crate is a WIP.")]

use crate::api::GraphBackend;

mod api;
mod fields;

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

impl<'a> GraphBackend for Graph<'a> {
    type Vertex = Vertex<'a>;
    type Arc = Arc<'a>;
    type Error = GraphError;

    const ARC_ALLOCS: usize = 102;
    const EXTRA_N: usize = 4;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n + Self::EXTRA_N);
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

    fn get(&self, idx: usize) -> Option<&Self::Vertex> {
        self.vertices.get(idx)
    }
    fn get_mut(&mut self, idx: usize) -> Option<&mut Self::Vertex> {
        self.vertices.get_mut(idx)
    }
}

struct GraphError(GraphErrorReason);

enum GraphErrorReason {
    NoVerticesInGraph,
}

#[cfg(test)]
mod tests {
    use algorist_grapht_macros::add;

    use super::*;

    #[test]
    fn it_works() {
        let mut graph = Graph::new(10);

        graph.get_mut(2).and_insert_arc();
        graph.get_mut(2).new_arc(graph.get_vert(3));

        // TODO: implement a macro that lets me access each field more
        // ergonomically inside of the function.
        #[cfg_attr(not(doc), add)]
        fn planar_graph<T>(_: &T)
        where
            T: GraphBackend + Fields<String, 2>,
            T::Vertex: Fields<u32, 1>,
        {
            <T as Field<String, 0>>::get();
            <T::Vertex as Field<u32, 0>>::get();
        }
    }
}
