use std::{borrow::Borrow, error::Error, fmt::Debug};

use num_traits::AsPrimitive;

use crate::private::Sealed;

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

    type Error: Error;

    fn new<T: AsPrimitive<usize>>(n: T) -> Result<Self, Self::Error>;

    fn cmd_mut<T: CommandMut<U, Self>, U>(&mut self, cmd: T) -> U {
        cmd.execute(self)
    }
    fn cmd<T: Command<U, Self>, U>(&self, cmd: T) -> U {
        cmd.execute(self)
    }
}

pub(crate) trait VertexIterExt<'a, G: GraphBackend + 'a> {
    fn iter(&'a self) -> impl Iterator<Item: 'a, Item = &'a G::Vertex>;
    fn iter_mut(&'a mut self) -> impl Iterator<Item: 'a, Item = &'a mut G::Vertex>;
}

pub(crate) trait IdExt {
    type Id;

    fn get_id<T: ?Sized>(&self) -> &T
    where
        Self::Id: Borrow<T>;

    fn set_id_with<T: Into<Self::Id>>(&mut self, other_fn: impl FnOnce() -> T);
    fn set_id<T: Into<Self::Id>>(&mut self, other: T) {
        self.set_id_with(|| other.into());
    }
}

pub(crate) trait Field<T, const N: usize> {
    fn get_field<Q>(&self) -> &Q
    where
        T: Borrow<Q>;
    fn set_field<Q: Into<T>>(&mut self, other: Q);
}

pub(crate) trait FieldsExt<T, const N: usize>: Sealed {}

pub(crate) trait Command<T, U: GraphBackend> {
    fn execute(self, graph: &U) -> T;
}

pub(crate) trait CommandMut<T, U: GraphBackend> {
    fn execute(self, graph: &mut U) -> T;
}
