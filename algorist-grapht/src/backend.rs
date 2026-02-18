use std::{
    alloc::AllocError,
    fmt::{Display, Formatter},
    marker::PhantomData,
    rc::Rc,
};

use num_traits::cast::AsPrimitive;
use thiserror::Error;

use crate::api::GraphBackend;

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

#[derive(Debug, Error)]
#[error("failed to allocate auxiliary memory")]
pub(crate) struct CloneShallowError;

#[derive(Debug, Error)]
pub(crate) enum IterMutError {
    #[error("failed to allocate auxiliary memory")]
    AllocFailed,
    #[error("vertex with index {0} is not uniquely owned")]
    NonUniqueOwnersip(usize),
}

impl Graph {
    const EXTRA_N: usize = 4;

    pub(crate) fn clone_shallow(&self) -> Result<Graph, CloneShallowError> {
        Ok(Self {
            vertices: self.vertices.iter().fold(
                Vec::try_with_capacity(self.vertices.len()).map_err(|_| CloneShallowError)?,
                |mut container, ptr| {
                    container.push(Rc::clone(ptr));

                    container
                },
            ),
            arcs: self.arcs.iter().fold(
                Vec::try_with_capacity(self.arcs.len()).map_err(|_| CloneShallowError)?,
                |mut container, ptr| {
                    container.push(Rc::clone(ptr));

                    container
                },
            ),
        })
    }

    pub(crate) fn try_iter_mut(&mut self) -> Result<IterMut<'_>, IterMutError> {
        let len = self.vertices.len();

        Ok(IterMut {
            container: self.vertices.iter_mut().enumerate().try_fold(
                Vec::try_with_capacity(len).map_err(|_| IterMutError::AllocFailed)?,
                |mut container, (idx, ptr)| {
                    container.push(
                        &raw mut *Rc::get_mut(ptr).ok_or(IterMutError::NonUniqueOwnersip(idx))?,
                    );

                    Ok(container)
                },
            )?,
            idx: None,
            _marker: PhantomData,
        })
    }
}

pub(crate) struct IterMut<'a> {
    container: Vec<*mut Vertex>,
    idx: Option<usize>,
    _marker: PhantomData<&'a mut Vertex>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        match self.idx {
            None => {
                self.idx = Some(0);
                self.container.first().map(|ptr| unsafe { &mut **ptr })
            }
            Some(ref mut idx) => {
                *idx += 1;
                self.container.get(*idx).map(|ptr| unsafe { &mut **ptr })
            }
        }
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

pub(crate) mod cmds {}
