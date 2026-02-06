pub(crate) trait GraphBackend
where
    Self: Sized,
    Self::Indexer: IndexerExt,
    Self::Arc: ArcExt<Self>,
{
    type Vertex;
    type Arc;

    type Indexer;
    type Error;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn new_arc(&mut self, other: Self::Indexer) -> &Self::Arc;

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer>;
}

pub(crate) trait IndexerExt {
    fn get(&self) -> Self;
}

pub(crate) trait ArcExt<T>
where
    T: GraphBackend,
{
    fn set_dst(&self, other: &T::Vertex) -> Option<()>;
}
