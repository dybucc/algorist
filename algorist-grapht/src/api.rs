use std::{
    error::Error,
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use num_traits::AsPrimitive;

pub(crate) mod routines;

// TODO: consider further separating the API for graph backend implementors into
// a separate crate, such that utility methods may be exposed to users of that
// crate that should otherwise not be allowed to users of the graph generative
// routines.

// TODO: report to the rust github that the Error associated type fails to
// resolve when implementing the trait on a specific type only when using a
// `where` clause and not straightforward supertrait syntax.

pub(crate) trait GraphBackend: Sized {
    type Vertex;
    type Arc;

    type Indexer;
    type Error: Error + Display;

    fn new<T: AsPrimitive<usize>>(n: T) -> Result<Self, Self::Error>;

    fn select<R: RangeBounds<Q>, Q: Into<Self::Indexer>>(&self, range: R) -> Select<Self::Indexer>;

    fn cmd_mut<T: CommandMut<U, Self>, U>(&mut self, cmd: T) -> U {
        cmd.execute(self)
    }
    fn cmd<T: Command<U, Self>, U>(&self, cmd: T) -> U {
        cmd.execute(self)
    }
}

pub(crate) trait Command<T, U: GraphBackend> {
    fn execute(self, graph: &U) -> T;
}

pub(crate) trait CommandMut<T, U: GraphBackend> {
    fn execute(self, graph: &mut U) -> T;
}

pub(crate) struct Select<T> {
    src: Bound<T>,
    dst: Bound<T>,
}
