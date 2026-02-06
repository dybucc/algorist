#![expect(dead_code, reason = "The crate is a WIP.")]

use std::sync::Arc as ArcPtr;

use crate::api::{ArcExt, GraphBackend, IndexerExt};

mod api;
mod fields;
mod private {
    pub(crate) trait Sealed {}
}

const ARC_ALLOCS: usize = 102;
const EXTRA_N: usize = 4;

#[derive(Debug)]
struct Arc {
    tip: ArcPtr<Vertex>,
    id: Option<usize>,
}

impl PartialEq for Arc {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug)]
struct Vertex {
    arcs: Vec<ArcPtr<Arc>>,
    // TODO: get UUID generation working
    id: Option<usize>,
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug)]
struct Graph {
    vertices: Vec<ArcPtr<Vertex>>,
    arcs: Vec<ArcPtr<Arc>>,
    n: usize,
    m: usize,
    // TODO: get UUID generation working
    id: Option<usize>,
}

struct GraphError(GraphErrorReason);

enum GraphErrorReason {
    NoVerticesInGraph,
}

impl IndexerExt for usize {
    fn get(&self) -> Self {
        *self
    }
}

impl<T> ArcExt<T> for Arc
where
    T: GraphBackend,
{
    fn set_dst(&self, other: &<T as GraphBackend>::Vertex) -> Option<()> {
        todo!();
    }
}

impl GraphBackend for Graph {
    type Vertex = Vertex;
    type Arc = Arc;

    type Indexer = usize;
    type Error = GraphError;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n + EXTRA_N);
                output.resize_with(n, || {
                    ArcPtr::new(Vertex {
                        arcs: Vec::new(),
                        id: None,
                    })
                });

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

    fn new_arc(&mut self, src: Self::Indexer) -> &Self::Arc {
        self.arcs.push(ArcPtr::new(Arc {
            tip: ArcPtr::clone(&self.vertices[src]),
            id: None,
        }));
        self.arcs.last().expect(
            "the `arcs` arena just allocated a new element so accessing it should be infallible",
        )
    }

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer> {
        self.vertices.iter().position(|inner| **inner == *elem)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused, reason = "WIP.")]
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
