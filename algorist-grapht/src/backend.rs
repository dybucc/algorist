use std::{fmt::Display, mem::MaybeUninit, rc::Rc};

use bumpalo::Bump;
use num_traits::{PrimInt, Unsigned};
use thiserror::Error;

use crate::api::GraphBackend;

#[derive(Debug)]
pub(crate) struct Arc {
    tip: Option<Rc<Vertex>>,
}

#[derive(Debug)]
pub(crate) struct Vertex {
    arcs: Option<Vec<Rc<Arc>>>,
}

#[derive(Debug)]
pub(crate) struct Graph<'a> {
    vertices: Vec<Rc<Vertex>, &'a Bump>,
    arcs: Vec<Rc<Arc>, &'a Bump>,
    arena_ref: &'a Bump,
    arena: Bump,
    n: usize,
    m: usize,
}

impl Graph<'_> {
    const EXTRA_N: usize = 4;
}

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("failed to create graph: {0}")]
    GraphCreationError(#[from] GraphCreationError),
}

#[derive(Debug, Error)]
pub enum GraphCreationError {
    #[cfg(not(doc))]
    #[expect(
        private_interfaces,
        reason = "`AllocErrorSrc` is meant to provide a private error representation with \
                 call-site information."
    )]
    #[error("failed to allocate requested memory: {0}")]
    AllocError(#[from] AllocErrorSrc),

    #[error("failed to parse input number of vertices")]
    ParseIntError,

    #[cfg(doc)]
    #[error("")]
    AllocError(),
}

#[derive(Debug, Error)]
pub(crate) enum AllocErrorSrc {
    #[error("allocation of arena blocks failed")]
    ArenaAlloc,

    #[cfg(not(doc))]
    #[error("allocation of {} {} failed", .0.0, .0.1)]
    ItemInArena(ItemInArena),

    #[cfg(doc)]
    #[error("")]
    ItemInArena(),
}

impl From<ItemInArena> for AllocErrorSrc {
    fn from(value: ItemInArena) -> Self {
        Self::ItemInArena(value)
    }
}

#[derive(Debug)]
pub(crate) struct ItemInArena(pub(crate) usize, pub(crate) ArenaItemType);

#[derive(Debug)]
pub(crate) enum ArenaItemType {
    Vert,
    Arc,
}

impl Display for ArenaItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ArenaItemType::{Arc, Vert};

        match self {
            Vert => write!(f, "vertices"),
            Arc => write!(f, "arcs"),
        }
    }
}

impl GraphBackend for Graph<'_> {
    // TODO: see into providing a borrowed view into vertices and/or arcs with
    //       a different data type.
    type BorrowedVertex = Vertex;
    type BorrowedArc = Arc;

    type Vertex = Vertex;
    type Arc = Arc;

    type Magnitude = usize;

    type Error = GraphError;

    default fn new<R>(n: R) -> <Self as GraphBackend>::Result<Self>
    where
        R: PrimInt + Unsigned,
    {
        let n = n.to_usize().ok_or(GraphCreationError::ParseIntError)?;
        let mut graph: MaybeUninit<Self> = MaybeUninit::uninit();
        let handle = graph.as_mut_ptr();

        unsafe {
            (&raw mut (*handle).arena).write(
                Bump::try_with_capacity((n + Graph::EXTRA_N) * size_of::<Self::Vertex>())
                    .map_err(|_| GraphCreationError::from(AllocErrorSrc::ArenaAlloc))?,
            );
            (&raw mut (*handle).arena_ref).write((&raw const (*handle).arena).as_ref_unchecked());

            (&raw mut (*handle).vertices).write(
                Vec::try_with_capacity_in(
                    n + Graph::EXTRA_N,
                    (&raw const (*handle).arena_ref).read(),
                )
                .map_err(|_| {
                    GraphCreationError::from(AllocErrorSrc::from(ItemInArena(
                        n,
                        ArenaItemType::Vert,
                    )))
                })?,
            );
            (&raw mut (*handle).arcs).write(Vec::new_in((&raw const (*handle).arena_ref).read()));

            (&raw mut (*handle).n).write(n);
            (&raw mut (*handle).m).write(0);
        }

        Ok(unsafe { graph.assume_init() })
    }

    default fn n(&self) -> Self::Magnitude {
        self.n
    }
    default fn m(&self) -> Self::Magnitude {
        self.m
    }
}
