use std::rc::Rc;

use bumpalo::Bump;
use thiserror::Error;

use crate::api::GraphBackend;

#[derive(Debug)]
pub(crate) struct Arc<T> {
    tip: Option<Rc<Vertex<T>>>,
}

#[derive(Debug)]
pub(crate) struct Vertex<T> {
    arcs: Option<Vec<Rc<Arc<T>>>>,
    inner: T,
}

impl<T> PartialEq for Vertex<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug)]
pub(crate) struct Graph<T> {
    vertices: BumpVec<Rc<Vertex<T>>>,
    arcs: BumpVec<Rc<Arc<T>>>,
    arena: Bump,
    n: usize,
    m: usize,
}

impl<T> Graph<T> {
    const EXTRA_N: usize = 4;
}

#[derive(Error, Debug)]
pub(crate) enum GraphError {
    #[error("allocation error: {0}")]
    AllocError(#[from] AllocError),
}

#[derive(Debug, Error)]
#[error("failed to allocate requested memory during {} allocation", .0)]
pub(crate) struct AllocError(AllocErrorSrc);

#[derive(Debug, Error)]
enum AllocErrorSrc {
    #[error("arc")]
    ArcAlloc,
    #[error("vertex")]
    VertAlloc,
}

impl<T> GraphBackend for Graph<T>
where
    T: Default,
{
    type Vertex = Rc<Vertex<T>>;
    type Arc = Rc<Arc<T>>;

    type Error = GraphError;

    default fn new(n: usize) -> <Self as GraphBackend>::Result<Self> {
        let arena = Bump::try_with_capacity(n + Graph::EXTRA_N)?;
        Ok(Graph { vertices: {
            let output
        }, arcs: (), arena, n, m: () })
    }

    default fn n(&self) -> usize {
        self.n
    }
    default fn m(&self) -> usize {
        self.m
    }
}
