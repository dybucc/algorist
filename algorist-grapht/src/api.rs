pub(crate) trait GraphBackend {
    type Vertex;
    type Arc;
    type Error;

    const ARC_ALLOCS: usize;
    const EXTRA_N: usize;

    fn new(n: usize) -> Self;

    fn n(&self) -> usize;
    fn m(&self) -> usize;

    fn get(&self, idx: usize) -> Option<&Self::Vertex>;
    fn get_mut(&mut self, idx: usize) -> Option<&mut Self::Vertex>;
}
