use std::error::Error;

pub(crate) trait GraphBackend
where
    Self: Sized,
    Self::Indexer: IndexerExt,
    Self::Error: Error,
{
    type Vertex;
    type Arc;

    type Indexer;
    type Error;

    type Result<T> = Result<T, Self::Error>;

    fn new(n: usize) -> Self::Result<Self>;

    fn n(&self) -> usize;
    fn m(&self) -> usize;
}

pub(crate) trait IndexerExt {
    fn get(&self) -> Self;
}
