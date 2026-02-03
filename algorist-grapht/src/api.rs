pub(crate) trait GraphBackend
where
    Self::Indexer: IndexerExt,
    Option<Self::VertexEntry>: VertexEntryExt,
{
    type Vertex;
    type Arc;

    type VertexEntry;

    type Indexer;
    type Error;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn get(&self, idx: Self::Indexer) -> Option<Self::VertexEntry>;
    fn get_mut(&mut self, idx: Self::Indexer) -> Option<Self::VertexEntry>;

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer>;
}

pub(crate) trait IndexerExt {
    fn get(&self) -> Self;
}

pub(crate) trait VertexEntryExt {
    fn and_insert_arc(&mut self, f: impl FnMut(&mut Self, Self));
}
