use std::{borrow::Borrow, error::Error};

use num_traits::{PrimInt, Unsigned};

pub(crate) trait GraphBackend
where
    Self: Sized,
    Self::Error: Error,
    Self::Vertex: Borrow<Self::BorrowedVertex>,
    Self::Arc: Borrow<Self::BorrowedArc>,
    Self::Magnitude: PrimInt + Unsigned,
{
    type BorrowedVertex;
    type BorrowedArc;

    type Vertex;
    type Arc;

    type Magnitude;

    type Error;
    type Result<T> = Result<T, <Self as GraphBackend>::Error>;

    fn new<T>(n: T) -> <Self as GraphBackend>::Result<Self>
    where
        T: PrimInt + Unsigned;

    fn n(&self) -> Self::Magnitude;
    fn m(&self) -> Self::Magnitude;
}
