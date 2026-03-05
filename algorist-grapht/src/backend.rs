use std::{
    alloc::AllocError,
    any::{Any, TypeId},
    borrow::{Borrow, BorrowMut},
    collections::{TryReserveError, hash_map::Entry},
    fmt::{Display, Formatter},
    marker::PhantomData,
    num::NonZeroIsize,
    ptr,
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
    pub(crate) tip: Option<Rc<Vertex>>,
    pub(crate) id: String,
}

impl PartialEq for Arc {
    fn eq(&self, other: &Self) -> bool {
        matches!((&self.tip, &other.tip), (Some(tip1), Some(tip2)) if Rc::ptr_eq(tip1, tip2))
    }
}

#[derive(Debug)]
pub(crate) struct Vertex {
    pub(crate) arcs: Vec<Rc<Arc>>,
    pub(crate) fields: FieldBuilder,
    pub(crate) id: String,
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
    pub(crate) vertices: Vec<Rc<Vertex>>,
    pub(crate) id: String,
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
            len: self.vertices.len(),
            idx: None,
            graph: self,
        }
    }

    pub(crate) fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            len: self.vertices.len(),
            idx: None,
            graph: self,
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
    pub(crate) len: usize,
    pub(crate) idx: Option<usize>,
    pub(crate) graph: &'a mut Graph,
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
            }
        }

        self.graph
            .vertices
            .get_mut(unsafe { self.idx.unwrap_unchecked() })
            .map(|ptr| unsafe { Rc::as_ptr(ptr).cast_mut().as_mut_unchecked() })
    }
}

pub(crate) struct Iter<'a> {
    pub(crate) len: usize,
    pub(crate) idx: Option<usize>,
    pub(crate) graph: &'a Graph,
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
            }
        }

        self.graph
            .vertices
            .get(unsafe { self.idx.unwrap_unchecked() })
            .map(|ptr| unsafe { Rc::as_ptr(ptr).as_ref_unchecked() })
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
                            fields: FieldBuilder::default(),
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

impl<T, const N: usize> FieldsExt<T, N> for Vertex
where
    for<'a> T: 'a,
{
    type Error = TryReserveError;

    fn chfield<'a, Q: 'a>(&mut self) -> Result<[&mut Q; N], Self::Error>
    where
        T: BorrowMut<Q> + Default + 'a,
    {
        match self.fields.0.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                let len = entry.len();
                if len < N {
                    entry.try_reserve_exact(N)?;
                    (len..N).for_each(|_| {
                        entry.push({
                            let input: Box<dyn Any> = Box::new(T::default());

                            input
                        });
                    });
                }
                let mut output: [*mut Q; N] = [ptr::null_mut(); N];
                entry.iter_mut().enumerate().take(N).for_each(|(i, ty)| {
                    // SAFETY: all elements `ty` in `entry` are of type `T` by
                    // virtue of hashing from `T`'s `TypeId` to the bucket of
                    // values `ty` of type `T`.
                    output[i] = unsafe { ty.downcast_unchecked_mut::<T>().borrow_mut() }
                });

                // SAFETY: the pointer actually points to the underlying value
                // behind the `Box<dyn Any>` of the hashmap `entry` is sourced
                // from, so producing a reference to it is sound.
                Ok(output.map(|ty| unsafe { &mut *ty }))
            }
            Entry::Vacant(key) => {
                let entry = key.insert({
                    let mut input = Vec::try_with_capacity(N)?;
                    input.resize_with(N, || {
                        let out: Box<dyn Any> = Box::new(T::default());

                        out
                    });

                    input
                });
                let mut output: [*mut Q; N] = [ptr::null_mut(); N];
                entry.iter_mut().enumerate().for_each(|(i, ty)| {
                    // SAFETY: all elements `ty` in `entry` are of type `T`
                    // because all elements pushed onto the new bucket are of
                    // type `T`.
                    output[i] = unsafe { ty.downcast_unchecked_mut::<T>().borrow_mut() }
                });

                // SAFETY: the pointer actually points to the underlying value
                // behind the `Box<dyn Any>` of the hashmap `entry` is sourced
                // from, so producing a reference to it is sound.
                Ok(output.map(|ty| unsafe { &mut *ty }))
            }
        }
    }

    fn chfield_with<'a, Q: 'a, R: Into<T>>(
        &mut self,
        function: impl Fn() -> R,
    ) -> Result<[&mut Q; N], Self::Error>
    where
        T: BorrowMut<Q> + 'a,
    {
        match self.fields.0.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                let len = entry.len();
                if len < N {
                    entry.try_reserve_exact(N)?;
                    (len..N).for_each(|_| {
                        entry.push({
                            let input: Box<dyn Any> = Box::new(function().into());

                            input
                        });
                    });
                }
                let mut output: [*mut Q; N] = [ptr::null_mut(); N];
                entry.iter_mut().enumerate().take(N).for_each(|(i, ty)| {
                    // SAFETY: all elements `ty` in `entry` are of type `T` by
                    // virtue of hashing from `T`'s `TypeId` to the bucket of
                    // values `ty` of type `T`.
                    output[i] = unsafe { ty.downcast_unchecked_mut::<T>().borrow_mut() }
                });

                // SAFETY: the pointer actually points to the underlying value
                // behind the `Box<dyn Any>` of the hashmap `entry` is sourced
                // from, so producing a reference to it is sound.
                Ok(output.map(|ty| unsafe { &mut *ty }))
            }
            Entry::Vacant(key) => {
                let entry = key.insert({
                    let mut input = Vec::try_with_capacity(N)?;
                    input.resize_with(N, || {
                        let out: Box<dyn Any> = Box::new(function().into());

                        out
                    });

                    input
                });
                let mut output: [*mut Q; N] = [ptr::null_mut(); N];
                entry.iter_mut().enumerate().for_each(|(i, ty)| {
                    // SAFETY: all elements `ty` in `entry` are of type `T`
                    // because all elements pushed onto the new bucket are of
                    // type `T`.
                    output[i] = unsafe { ty.downcast_unchecked_mut::<T>().borrow_mut() }
                });

                // SAFETY: the pointer actually points to the underlying value
                // behind the `Box<dyn Any>` of the hashmap `entry` is sourced
                // from, so producing a reference to it is sound.
                Ok(output.map(|ty| unsafe { &mut *ty }))
            }
        }
    }
}

impl Board for Graph {
    type GraphId = String;
    type VertexId = String;
    type ArcId = String;
}

pub(crate) mod cmds {}
