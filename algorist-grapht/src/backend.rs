use std::{ascii::Char, collections::TryReserveErrorKind, sync::Arc as ArcPtr};

use thiserror::Error;

use crate::api::{GraphBackend, IndexerExt};

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
    // TODO: get UID generation working
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
    // TODO: get UID generation working
    id: Option<usize>,
}

impl Graph {
    const EXTRA_N: usize = 4;
}

#[derive(Error, Debug)]
enum GraphError {
    #[error("graph creation failed: {0}")]
    GraphCreationError(GraphCreationError),
}

impl From<GraphCreationError> for GraphError {
    fn from(value: GraphCreationError) -> Self {
        Self::GraphCreationError(value)
    }
}

#[derive(Error, Debug)]
enum GraphCreationError {
    #[error("allocation failed during {src}: {reason}", src = crate::parse_ascii_char(.src))]
    AllocationFailed {
        reason: AllocationReason,
        src: Box<[Char]>,
    },
}

#[derive(Error, Debug)]
enum AllocationReason {
    #[error("capacity surpasses {max}", max = isize::MAX)]
    CapacityOverflow,
    #[error("allocator memory request failed")]
    AllocatorError,
}

impl IndexerExt for usize {
    fn get(&self) -> Self {
        *self
    }
}

impl GraphBackend for Graph {
    type Vertex = ArcPtr<Vertex>;
    type Arc = ArcPtr<Arc>;

    type Indexer = usize;
    type Error = GraphError;

    fn new(n: usize) -> Self::Result<Self> {
        Ok(Graph {
            vertices: {
                let mut output = Vec::new();
                output
                    .try_reserve(n + Graph::EXTRA_N)
                    .map(|()| {
                        output.resize_with(n, || {
                            ArcPtr::new(Vertex {
                                arcs: Vec::new(),
                                id: None,
                            })
                        });
                    })
                    .map_err(|elem| match elem.kind() {
                        TryReserveErrorKind::CapacityOverflow => {
                            GraphCreationError::AllocationFailed {
                                reason: AllocationReason::CapacityOverflow,
                                src: crate::error!("vertex arena allocation"),
                            }
                        }
                        TryReserveErrorKind::AllocError { .. } => {
                            GraphCreationError::AllocationFailed {
                                reason: AllocationReason::AllocatorError,
                                src: crate::error!("vertex arena allocation"),
                            }
                        }
                    })?;

                output
            },
            arcs: Vec::new(),
            n,
            m: 0,
            id: None,
        })
    }

    fn n(&self) -> usize {
        self.n
    }
    fn m(&self) -> usize {
        self.m
    }
}
