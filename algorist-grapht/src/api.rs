use std::{
  borrow::{Borrow, BorrowMut},
  error::Error,
};

use num_traits::AsPrimitive;

pub(crate) mod routines;

// TODO: consider further separating the API for graph backend implementors into
// a separate crate, such that utility methods may be exposed to users of that
// crate that should otherwise not be allowed to users of the graph generative
// routines.

// TODO: report to the rust github that the Error associated type fails to
// resolve when implementing the trait on a specific type only when using a
// `where` clause and not supertrait syntactic sugar.

pub(crate) trait GraphBackend: Sized {
  type Vertex;
  type Arc;

  type Error: Error;

  fn new<T: AsPrimitive<usize>>(
    n: T,
  ) -> Result<Self, <Self as GraphBackend>::Error>;
}

pub(crate) trait VertexIterExt<'a, G: GraphBackend + 'a> {
  type SharedIter: Iterator<Item: 'a, Item = &'a G::Vertex> + 'a;
  type ExclusiveIter: Iterator<Item: 'a, Item = &'a mut G::Vertex> + 'a;

  fn iter(&'a self) -> <Self as VertexIterExt<'a, G>>::SharedIter;
  fn iter_mut(&'a mut self) -> <Self as VertexIterExt<'a, G>>::ExclusiveIter;
}

// NOTE: the order of iteration on the implementation of `VertexIterExt` is the
// one used for the indices in the default backend's implementation of
// `ArcAddExt`. That's the reason why it's always a `usize`. Right now, the
// implementation uses the potentially costly `count()` method on the returned
// iterator whenever it requires access to the indices of vertices in a graph,
// but compile-time reflection could improve that if the `TypeId` of the
// returned iterator could be determined to implement `ExactSizeIterator`; that
// should allow calling `len()` at the start of iteration, which should yield
// all elements about to be iterated over.
pub(crate) trait ArcAddExt {
  type Error: Error;

  fn new_arc(
    &mut self,
    src: usize,
    dst: usize,
  ) -> Result<(), <Self as ArcAddExt>::Error>;

  fn new_edge(
    &mut self,
    one: usize,
    other: usize,
  ) -> Result<(), <Self as ArcAddExt>::Error> {
    (self.new_arc(one, other)?, self.new_arc(other, one)).1
  }
}

pub(crate) trait IdExt {
  type Id;

  fn get_id<T: ?Sized>(&self) -> &T
  where
    <Self as IdExt>::Id: Borrow<T>;

  fn set_id_with<T: Into<<Self as IdExt>::Id>>(
    &mut self,
    other_fn: impl FnOnce() -> T,
  );

  fn set_id<T: Into<<Self as IdExt>::Id>>(&mut self, other: T) {
    self.set_id_with(|| other);
  }
}

pub(crate) trait FieldsExt<T, const N: usize> {
  type Error: Error;

  fn chfield<'a, Q: 'a>(
    &mut self,
  ) -> Result<[&mut Q; N], <Self as FieldsExt<T, N>>::Error>
  where
    T: BorrowMut<Q> + Default + 'a,
  {
    self.chfield_with(|| {
      Ok::<_, <Self as FieldsExt<T, N>>::Error>(<T as Default>::default())
    })
  }

  fn chfield_with<
    'a,
    Q: 'a,
    R: Into<T>,
    E: Into<<Self as FieldsExt<T, N>>::Error>,
  >(
    &mut self,
    function: impl FnMut() -> Result<R, E>,
  ) -> Result<[&mut Q; N], <Self as FieldsExt<T, N>>::Error>
  where
    T: BorrowMut<Q> + 'a;
}
