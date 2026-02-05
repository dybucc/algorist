#![expect(dead_code, reason = "The crate is a WIP.")]

use std::marker::PhantomData;

use crate::api::{GraphBackend, IndexerExt, MutVertexEntryExt, SharedVertexEntryExt};

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
    id: Option<usize>,
}

impl PartialEq for Arc<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug)]
struct Vertex<'a> {
    arcs: Vec<&'a Arc<'a>>,
    // TODO: get UUID generation working
    id: Option<usize>,
}

impl PartialEq for Vertex<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug)]
struct Graph<'a> {
    vertices: Vec<Vertex<'a>>,
    arcs: Vec<Arc<'a>>,
    n: usize,
    m: usize,
    // TODO: get UUID generation working
    id: Option<usize>,
}

struct GraphError(GraphErrorReason);

enum GraphErrorReason {
    NoVerticesInGraph,
}

struct MutVertexEntry<'a> {
    graph: *mut Graph<'a>,
    inner: *mut Vertex<'a>,
    _marker: PhantomData<&'a mut Vertex<'a>>,
}

struct SharedVertexEntry<'a> {
    graph: *const Graph<'a>,
    inner: *const Vertex<'a>,
    _marker: PhantomData<&'a Vertex<'a>>,
}

impl IndexerExt for usize {
    fn get(&self) -> Self {
        *self
    }
}

impl MutVertexEntryExt for MutVertexEntry<'_> {
    fn and_insert_arc(&mut self, other: Self) -> Option<()> {
        let handle = unsafe { &mut *self.graph };
        let vert_pos = handle
            .vertices
            .iter()
            .position(|elem| *elem == unsafe { self.inner.read() })?;
        (handle
            .arcs
            .iter()
            .any(|inner_arc| handle.vertices[vert_pos].arcs.contains(&inner_arc)))
        .then(|| {
            handle.arcs.push(Arc {
                tip: unsafe { &*other.inner },
                id: None,
            });
            handle.vertices[vert_pos]
                .arcs
                .push(handle.arcs.last().unwrap());
        });
        Some(())
    }
}

impl SharedVertexEntryExt for SharedVertexEntry<'_> {}

impl<'a> GraphBackend for Graph<'a> {
    type Vertex = Vertex<'a>;
    type Arc = Arc<'a>;

    type MutVertexEntry = MutVertexEntry<'a>;
    type SharedVertexEntry = SharedVertexEntry<'a>;

    type Indexer = usize;
    type Error = GraphError;

    fn new(n: usize) -> Self {
        Graph {
            vertices: {
                let mut output = Vec::with_capacity(n + EXTRA_N);
                output.resize_with(n, || Vertex {
                    arcs: Vec::new(),
                    id: None,
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

    fn get(&self, idx: Self::Indexer) -> Option<Self::SharedVertexEntry> {
        todo!()
    }
    fn get_mut(&mut self, idx: Self::Indexer) -> Option<Self::MutVertexEntry> {
        let graph = &raw mut *self;
        self.vertices.get_mut(idx).map(|elem| {
            let inner = &raw mut *elem;
            MutVertexEntry {
                graph,
                inner,
                _marker: PhantomData,
            }
        })
    }

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer> {
        self.vertices.iter().position(|inner| inner == elem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut graph = Graph::new(10);
        // TODO: check that the vertices before and after insertion correspond.
        assert_eq!(graph.vertices, vec![]);
        graph
            .get_mut(3)
            .unwrap()
            .and_insert_arc(graph.get_mut(2).unwrap());
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
