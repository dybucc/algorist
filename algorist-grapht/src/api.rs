pub(crate) trait GraphBackend
where
    Self: Sized,
    Self::Indexer: IndexerExt,
    Self::MutVertexEntry: MutVertexEntryExt,
    Self::SharedVertexEntry: SharedVertexEntryExt,
{
    type Vertex;
    type Arc;

    type MutVertexEntry;
    type SharedVertexEntry;

    type Indexer;
    type Error;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn get(&self, idx: Self::Indexer) -> Option<Self::SharedVertexEntry>;
    fn get_mut(&mut self, idx: Self::Indexer) -> Option<Self::MutVertexEntry>;

    fn get_indexer(&self, elem: &Self::Vertex) -> Option<Self::Indexer>;
}

pub(crate) trait IndexerExt {
    fn get(&self) -> Self;
}

pub(crate) trait MutVertexEntryExt {
    fn and_insert_arc(&mut self, other: Self) -> Option<()>;
}

// TODO: find usecase in API for an immutable view into a vertex.
pub(crate) trait SharedVertexEntryExt {}
