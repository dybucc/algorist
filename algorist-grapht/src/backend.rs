use std::{
    alloc::AllocError,
    any::{Any, TypeId},
    borrow::Borrow,
    collections::hash_map::Entry,
    fmt::{Display, Formatter},
    marker::PhantomData,
    num::NonZeroIsize,
    rc::Rc,
};

use num_traits::cast::AsPrimitive;
use thiserror::Error;

use crate::{
    api::{
        Field, FieldsExt, GraphBackend, IdExt, VertexIterExt,
        routines::basic::board::{Board, BoardError},
    },
    fields::FieldBuilder,
};

#[derive(Debug)]
pub(crate) struct Arc {
    tip: Option<Rc<Vertex>>,
    id: String,
}

impl PartialEq for Arc {
    fn eq(&self, other: &Self) -> bool {
        matches!((&self.tip, &other.tip), (Some(tip1), Some(tip2)) if Rc::ptr_eq(tip1, tip2))
    }
}

#[derive(Debug)]
pub(crate) struct Vertex {
    arcs: Vec<Rc<Arc>>,
    fields: FieldBuilder,
    id: String,
}

impl IdExt for Vertex {
    type Id = String;

    fn get_id<T: ?Sized>(&self) -> &T
    where
        Self::Id: Borrow<T>,
    {
        self.id.borrow()
    }

    fn set_id_with<T: Into<Self::Id>>(&mut self, other_fn: impl FnOnce() -> T) {
        self.id = other_fn().into();
    }
}

#[derive(Debug)]
pub(crate) struct Graph {
    vertices: Vec<Rc<Vertex>>,
    id: String,
}

#[derive(Debug, Error)]
#[error("failed to allocate auxiliary memory")]
pub(crate) struct CloneShallowError;

#[derive(Debug, Error)]
pub(crate) enum TryIterMutError {
    #[error("failed to allocate auxiliary memory")]
    AllocFailed,
    #[error("vertex with index {0} is not uniquely owned")]
    NonUniqueOwnersip(usize),
}

impl Graph {
    const EXTRA_N: usize = 4;

    pub fn new(n: usize) -> Result<Self, GraphCreationError> {
        <Self as GraphBackend>::new(n)
    }

    pub fn board(
        n1: isize,
        n2: isize,
        n3: isize,
        n4: isize,
        piece: NonZeroIsize,
        wrap: isize,
        directed: isize,
    ) -> Result<Self, BoardError<Self>> {
        <Self as Board>::board(n1, n2, n3, n4, piece, wrap, directed)
    }

    pub(crate) fn clone_shallow(&self) -> Result<Graph, CloneShallowError> {
        Ok(Self {
            vertices: self.vertices.iter().fold(
                Vec::try_with_capacity(self.vertices.len()).map_err(|_| CloneShallowError)?,
                |mut container, ptr| {
                    container.push(Rc::clone(ptr));

                    container
                },
            ),
            id: String::new(),
        })
    }

    pub(crate) fn iter(&self) -> Iter<'_> {
        Iter {
            first: self.vertices.first().map(|ptr| &raw const *ptr),
            len: self.vertices.len(),
            idx: None,
            _marker: PhantomData,
        }
    }

    pub(crate) fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            first: self.vertices.first_mut().map(|ptr| &raw mut *ptr),
            len: self.vertices.len(),
            idx: None,
            _marker: PhantomData,
        }
    }

    pub(crate) fn try_iter_mut(&mut self) -> Result<TryIterMut<'_>, TryIterMutError> {
        let len = self.vertices.len();

        Ok(TryIterMut {
            container: self.vertices.iter_mut().enumerate().try_fold(
                Vec::try_with_capacity(len).map_err(|_| TryIterMutError::AllocFailed)?,
                |mut container, (idx, ptr)| {
                    container.push(
                        &raw mut *Rc::get_mut(ptr)
                            .ok_or(TryIterMutError::NonUniqueOwnersip(idx))?,
                    );

                    Ok(container)
                },
            )?,
            idx: None,
            _marker: PhantomData,
        })
    }
}

impl<'a> IntoIterator for &'a mut Graph {
    type Item = <IterMut<'a> as Iterator>::Item;
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub(crate) struct IterMut<'a> {
    first: Option<*mut Rc<Vertex>>,
    len: usize,
    idx: Option<usize>,
    _marker: PhantomData<&'a mut Vertex>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        match self.idx {
            None => {
                if self.len == 0 {
                    return None;
                }
                self.idx = Some(0);
            }
            Some(ref mut idx) => {
                if *idx == self.len - 1 {
                    return None;
                }
                *idx += 1;
                self.first = self.first.map(|ptr| unsafe { ptr.add(1) });
            }
        }

        self.first
            .as_ref()
            .map(|ptr| unsafe { &mut *Rc::as_ptr(&**ptr).cast_mut() })
    }
}

pub(crate) struct Iter<'a> {
    first: Option<*const Rc<Vertex>>,
    len: usize,
    idx: Option<usize>,
    _marker: PhantomData<&'a Vertex>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        match self.idx {
            None => {
                if self.len > 0 {
                    return None;
                }
                self.idx = Some(0);
            }
            Some(ref mut idx) => {
                if *idx == self.len - 1 {
                    return None;
                }
                *idx += 1;
                self.first = self.first.map(|ptr| unsafe { ptr.add(1) });
            }
        }

        self.first
            .as_ref()
            .map(|ptr| unsafe { &*Rc::as_ptr(&**ptr) })
    }
}

pub(crate) struct TryIterMut<'a> {
    container: Vec<*mut Vertex>,
    idx: Option<usize>,
    _marker: PhantomData<&'a mut Vertex>,
}

impl<'a> Iterator for TryIterMut<'a> {
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

    type Error = GraphCreationError;

    fn new<T: AsPrimitive<usize>>(n: T) -> Result<Graph, Self::Error> {
        let n = n.as_();

        Ok(Graph {
            vertices: (0..n)
                .try_fold(
                    Vec::try_with_capacity(n + Graph::EXTRA_N)
                        .map_err(|_| GraphCreationError::AllocError(AllocErrorSrc::ArenaAlloc))?,
                    |mut output, _| {
                        output.push(Rc::try_new(Vertex {
                            arcs: Vec::new(),
                            id: String::new(),
                        })?);

                        Ok::<_, AllocError>(output)
                    },
                )
                .map_err(|_| {
                    GraphCreationError::AllocError(AllocErrorSrc::ItemInArena(ItemInArena(
                        n,
                        ArenaItemType::Vert,
                    )))
                })?,
            id: String::new(),
        })
    }
}

impl IdExt for Graph {
    type Id = String;

    fn get_id<T: ?Sized>(&self) -> &T
    where
        Self::Id: Borrow<T>,
    {
        self.id.borrow()
    }

    fn set_id_with<T: Into<Self::Id>>(&mut self, other_fn: impl FnOnce() -> T) {
        self.id = other_fn().into();
    }
}

impl<'a> VertexIterExt<'a, Self> for Graph {
    type SharedIter = Iter<'a>;
    type ExclusiveIter = IterMut<'a>;

    fn iter(&'a self) -> Self::SharedIter {
        self.iter()
    }

    fn iter_mut(&'a mut self) -> Self::ExclusiveIter {
        self.iter_mut()
    }
}

impl Field<usize, 0> for Vertex {
    fn get_field<Q>(&self) -> &Q
    where
        usize: Borrow<Q>,
    {
        todo!()
    }

    fn set_field<Q: Into<usize>>(&mut self, other: Q) {
        todo!()
    }
}

impl Field<usize, 1> for Vertex {
    fn get_field<Q>(&self) -> &Q
    where
        usize: Borrow<Q>,
    {
        todo!()
    }

    fn set_field<Q: Into<usize>>(&mut self, other: Q) {
        todo!()
    }
}

impl Field<usize, 2> for Vertex {
    fn get_field<Q>(&self) -> &Q
    where
        usize: Borrow<Q>,
    {
        todo!()
    }

    fn set_field<Q: Into<usize>>(&mut self, other: Q) {
        todo!()
    }
}

impl<T: Default, const N: usize> FieldsExt<T, N> for Vertex
where
    for<'a> T: 'a,
{
    fn get_field<Q>(&mut self) -> [Q; N]
    where
        T: Borrow<Q>,
        for<'a> Q: 'a,
    {
        match self.fields.0.entry(TypeId::of::<Q>()) {
            Entry::Occupied(entry) => {
                let entry = entry.get_mut();
                if entry.len() < N {
                    entry.try_reserve_exact(N);
                }
            }
        }

        self.fields
            .0
            .entry(TypeId::of::<Q>())
            .and_modify(|ty_vec| {
                if ty_vec.len() < N {
                    (0..N).for_each(|_| {
                        ty_vec.push({
                            let input: Box<dyn Any> = Box::new(T::default());

                            input
                        })
                    });
                }
            })
            .or_insert_with(|| {
                let ty_vec = Vec::trywith();

                ty_vec
            });

        todo!()
    }

    fn set_field<Q: Into<T>>(&mut self, other: Q) {}
}

impl Board for Graph {
    type Id = String;
}

pub(crate) mod cmds {}
