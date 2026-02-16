use std::{
    alloc::AllocError,
    fmt::{Display, Formatter},
    rc::Rc,
    slice::IterMut,
};

use num_traits::cast::AsPrimitive;
use thiserror::Error;

use crate::api::{GraphBackend, Indexer};

#[derive(Debug)]
pub(crate) struct Arc {
    tip: Option<Rc<Vertex>>,
}

impl PartialEq for Arc {
    fn eq(&self, other: &Self) -> bool {
        matches!((&self.tip, &other.tip), (Some(tip1), Some(tip2)) if Rc::ptr_eq(tip1, tip2))
    }
}

#[derive(Debug)]
pub(crate) struct Vertex {
    arcs: Vec<Rc<Arc>>,
}

#[derive(Debug)]
pub(crate) struct Graph {
    vertices: Vec<Rc<Vertex>>,
    arcs: Vec<Rc<Arc>>,
}

impl Graph {
    const EXTRA_N: usize = 4;

    pub(crate) fn iter_mut(&mut self) -> IterMut<'_, Rc<Vertex>> {
        self.vertices.iter_mut()
    }
}

struct Iter {}

pub(crate) struct Index(pub(crate) Option<usize>);

impl From<usize> for Index {
    fn from(value: usize) -> Self {
        Self(Some(value))
    }
}

#[derive(Debug, Error)]
pub(crate) enum GraphCreationError {
    #[error("failed to allocate requested memory: allocation of {0} failed")]
    AllocError(AllocErrorSrc),
}

#[derive(Debug)]
pub(crate) enum AllocErrorSrc {
    ArenaAlloc,
    ItemInArena(ItemInArena),
}

impl Display for AllocErrorSrc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArenaAlloc => write!(f, "arena blocks"),
            Self::ItemInArena(item) => write!(f, "{} {}", item.0, item.1),
        }
    }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ArenaItemType::{Arc, Vert};

        match self {
            Vert => write!(f, "vertices"),
            Arc => write!(f, "arcs"),
        }
    }
}

impl GraphBackend for Graph {
    type Vertex = Vertex;
    type Arc = Arc;

    type CreationResult = Result<Graph, GraphCreationError>;

    fn new<T>(n: T) -> Self::CreationResult
    where
        T: AsPrimitive<usize>,
    {
        let n = n.as_();

        Ok(Graph {
            vertices: (0..n)
                .try_fold(
                    Vec::try_with_capacity(n + Graph::EXTRA_N)
                        .map_err(|_| GraphCreationError::AllocError(AllocErrorSrc::ArenaAlloc))?,
                    |mut output, _| {
                        output.push(Rc::try_new(Vertex { arcs: Vec::new() })?);

                        Ok::<_, AllocError>(output)
                    },
                )
                .map_err(|_| {
                    GraphCreationError::AllocError(AllocErrorSrc::ItemInArena(ItemInArena(
                        n,
                        ArenaItemType::Vert,
                    )))
                })?,
            arcs: Vec::new(),
        })
    }
}

pub(crate) mod cmds {
    use thiserror::Error;

    use crate::api::{CommandMut, GraphBackend, Indexer, Insertion};

    #[derive(Debug, Error)]
    pub(crate) enum InsertionError {}

    impl<'a, T, I, U> CommandMut<Result<(), InsertionError>> for Insertion<'a, T, I, U>
    where
        U: GraphBackend,
        T: Indexer<I>,
        I: Iterator<Item = &'a mut U::Vertex>,
        U::Vertex: 'a,
    {
        fn execute<R>(self, graph: &mut R) -> Result<(), InsertionError>
        where
            R: GraphBackend,
        {
            todo!()
        }
    }
}
