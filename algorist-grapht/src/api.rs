use std::{
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

use num_traits::AsPrimitive;
use thiserror::Error;

// TODO: consider further separating the API for graph backend implementors into
// a separate crate, such that utility methods may be exposed to users of that
// crate that should otherwise not be allowed to users of the graph generative
// routines.

pub(crate) trait GraphBackend
where
    Self: Sized,
{
    type Vertex;
    type Arc;

    type CreationResult;

    fn new<T>(n: T) -> Self::CreationResult
    where
        T: AsPrimitive<usize>;

    fn cmd_mut<T, U>(&mut self, cmd: T) -> U
    where
        T: CommandMut<U>,
    {
        cmd.execute(self)
    }
    fn cmd<T, U>(&self, cmd: T) -> U
    where
        T: Command<U>,
    {
        cmd.execute(self)
    }
}

pub(crate) trait Command<T> {
    fn execute<U>(self, graph: &U) -> T
    where
        U: GraphBackend;
}

pub(crate) trait CommandMut<T> {
    fn execute<U>(self, graph: &mut U) -> T
    where
        U: GraphBackend;
}

pub(crate) trait Indexer<I>
where
    I: Iterator,
{
    fn get(iter: &I, idx: &Self) -> I::Item;
}

pub(crate) enum Insertion<'a, T, I, U>
where
    U: GraphBackend,
    T: Indexer<I>,
    I: Iterator<Item = &'a mut U::Vertex>,
    U::Vertex: 'a,
{
    Arc(Select<'a, T, I, U>),
    Vertex,
}

pub(crate) struct Select<'a, T, I, U>
where
    U: GraphBackend,
    T: Indexer<I>,
    I: Iterator<Item = &'a mut U::Vertex>,
    U::Vertex: 'a,
{
    pub(crate) src: Bound<T>,
    pub(crate) dst: Bound<T>,
    pub(crate) iter: &'a I,
    pub(crate) _marker: PhantomData<U>,
}

#[derive(Debug, Error)]
pub(crate) enum SelectionError {
    #[error("missing source vertex")]
    SrcNotIncluded,
    #[error("missing destination vertex")]
    DstNotIncluded,
}

impl<'a, T, I, U> Select<'a, T, I, U>
where
    U: GraphBackend,
    T: Indexer<I>,
    I: Iterator<Item = &'a mut U::Vertex>,
    U::Vertex: 'a,
{
    fn src(&mut self) -> Result<I::Item, SelectionError> {
        if let Bound::Included(src) = &self.src {
            Ok(T::get(self.iter, src))
        } else {
            Err(SelectionError::SrcNotIncluded)
        }
    }

    fn dst(&self) -> Result<I::Item, SelectionError> {
        if let Bound::Included(dst) | Bound::Excluded(dst) = &self.dst {
            Ok(T::get(self.iter, dst))
        } else {
            Err(SelectionError::DstNotIncluded)
        }
    }
}

pub(crate) trait Selection<'a, U, T>
where
    Self: Iterator<Item = &'a mut T::Vertex> + Sized,
    T: GraphBackend,
    T::Vertex: 'a,
    U: Indexer<Self>,
{
    fn select<R>(&self, range: R) -> Select<'a, U, Self, T>
    where
        R: RangeBounds<U>;
}
