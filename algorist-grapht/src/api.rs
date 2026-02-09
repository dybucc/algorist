use std::error::Error;

pub(crate) trait GraphBackend
where
    Self: Sized,
    <Self as GraphBackend>::Error: Error,
{
    type Vertex;
    type Arc;

    type Error;
    type Result<T> = Result<T, <Self as GraphBackend>::Error>;

    fn new(n: usize) -> <Self as GraphBackend>::Result<Self>;

    fn n(&self) -> usize;
    fn m(&self) -> usize;
}
