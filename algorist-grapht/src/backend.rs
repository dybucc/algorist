use std::{
  alloc::AllocError,
  borrow::Borrow,
  debug_assert_matches,
  num::NonZeroIsize,
  rc::Rc,
};

use num_traits::cast::AsPrimitive;
use thiserror::Error;

use crate::{
  api::{
    ArcAddExt,
    GraphBackend,
    IdExt,
    VertexIterExt,
    routines::basic::board::{Board, BoardError},
  },
  backend::{arc::Arc, iter::Iter, iter_mut::IterMut, vertex::Vertex},
  fields::FieldBuilder,
};

pub(crate) mod arc;
pub(crate) mod iter;
pub(crate) mod iter_mut;
pub(crate) mod vertex;

#[derive(Debug)]
pub(crate) struct Graph {
  pub(crate) vertices: Vec<Rc<Vertex>>,
  pub(crate) fields:   FieldBuilder,
  pub(crate) id:       String,
}

impl Graph {
  const EXTRA_N: usize = 4;

  pub fn new(n: usize) -> Result<Self, GraphCreationError> {
    <Self as GraphBackend>::new(n)
  }

  pub fn board(
    n1: isize,
    n2: isize,
    n3: isize,
    n4: isize,
    piece: NonZeroIsize,
    wrap: isize,
    directed: bool,
  ) -> Result<Self, BoardError> {
    <Self as Board>::board(n1, n2, n3, n4, piece, wrap, directed)
  }

  pub(crate) fn iter(&self) -> Iter<'_> {
    Iter { len: self.vertices.len(), idx: None, graph: self }
  }

  pub(crate) fn iter_mut(&mut self) -> IterMut<'_> {
    IterMut { len: self.vertices.len(), idx: None, graph: self }
  }
}

impl<'a> IntoIterator for &'a Graph {
  type IntoIter = Iter<'a>;
  type Item = <Iter<'a> as Iterator>::Item;

  fn into_iter(self) -> Self::IntoIter { self.iter() }
}

impl<'a> IntoIterator for &'a mut Graph {
  type IntoIter = IterMut<'a>;
  type Item = <IterMut<'a> as Iterator>::Item;

  fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
}

impl GraphBackend for Graph {
  type Arc = Arc;
  type Error = GraphCreationError;
  type Vertex = Vertex;

  fn new<T: AsPrimitive<usize>>(n: T) -> Result<Graph, Self::Error> {
    let n = n.as_();

    Ok(Graph {
      vertices: (0..n)
        .try_fold(
          Vec::try_with_capacity(n + Graph::EXTRA_N).map_err(|_| {
            GraphCreationError::AllocError(AllocErrorSrc::ArenaAlloc)
          })?,
          |mut output, _| {
            output.push(Rc::try_new(Vertex {
              arcs:   Vec::new(),
              id:     String::new(),
              fields: FieldBuilder::default(),
            })?);

            Ok::<_, AllocError>(output)
          },
        )
        .map_err(|_| {
          GraphCreationError::AllocError(AllocErrorSrc::ItemInArena(
            n,
            ArenaItemType::Vert,
          ))
        })?,
      fields:   FieldBuilder::default(),
      id:       String::new(),
    })
  }
}

impl IdExt for Graph {
  type Id = String;

  fn get_id<T: ?Sized>(&self) -> &T
  where
    Self::Id: Borrow<T>,
  {
    self.id.borrow()
  }

  fn set_id_with<T: Into<Self::Id>>(&mut self, other_fn: impl FnOnce() -> T) {
    self.id = other_fn().into();
  }
}

impl<'a> VertexIterExt<'a, Self> for Graph {
  type ExclusiveIter = IterMut<'a>;
  type SharedIter = Iter<'a>;

  fn iter(&'a self) -> <Self as VertexIterExt<'a, Self>>::SharedIter {
    self.iter()
  }

  fn iter_mut(
    &'a mut self,
  ) -> <Self as VertexIterExt<'a, Self>>::ExclusiveIter {
    self.iter_mut()
  }
}

impl ArcAddExt for Graph {
  type Error = ArcAddError;

  fn new_arc(
    &mut self,
    src: usize,
    dst: usize,
  ) -> Result<(), <Self as ArcAddExt>::Error> {
    #![expect(clippy::unit_arg, reason = "Beauty comes at cost.")]

    debug_assert_matches!(
      (
        (0..self.vertices.len()).contains(src.borrow()),
        (0..self.vertices.len()).contains(dst.borrow())
      ),
      (true, true)
    );

    // SAFETY: the only place from which this routine gets called are the traits
    // implementing GraphBase functionality, which themselves only run in debug
    // until they work. On those runs, the above assertion should serve as a
    // form of bounds checking.
    Ok(
      unsafe {
        (
          (*Rc::as_ptr(self.vertices.get_unchecked(src)).cast_mut())
            .arcs
            .try_reserve(1)
            .map_err(|_| {
              ArcAddError::AuxiliaryAlloc(AllocFailureSrc::ArcCreation)
            })?,
          (*Rc::as_ptr(self.vertices.get_unchecked(src)).cast_mut()).arcs.push(
            Arc {
              tip:    Rc::clone(self.vertices.get_unchecked(dst)).into(),
              fields: FieldBuilder::default(),
              id:     String::new(),
            }
            .into(),
          ),
        )
      }
      .0,
    )
  }
}

impl Board for Graph {
  type ArcId = String;
  type GraphId = String;
  type VertexId = String;
}

#[derive(Debug, Error)]
pub(crate) enum GraphCreationError {
  #[error(
    "failed to allocate requested memory: allocation of {} failed",
    match .0 {
      AllocErrorSrc::ArenaAlloc => "arena blocks".to_string(),
      AllocErrorSrc::ItemInArena(num, ty) =>
        format!(
          "{num} {}",
          match ty {
            ArenaItemType::Vert if *num > 1 => "vertices",
            ArenaItemType::Vert => "vertex",
            ArenaItemType::Arc if *num > 1 => "arcs",
            ArenaItemType::Arc => "arc"
          }
        )
    }
  )]
  AllocError(AllocErrorSrc),
}

#[derive(Debug)]
pub(crate) enum AllocErrorSrc {
  ArenaAlloc,
  ItemInArena(usize, ArenaItemType),
}

#[derive(Debug)]
pub(crate) enum ArenaItemType {
  Vert,
  Arc,
}

#[derive(Debug, Error)]
pub(crate) enum ArcAddError {
  #[error(
    "auxiliary memory allocation failed while {}",
    match .0 {
      AllocFailureSrc::ArcCreation => "creating new arc",
    }
  )]
  AuxiliaryAlloc(AllocFailureSrc),
}

#[derive(Debug)]
pub(crate) enum AllocFailureSrc {
  ArcCreation,
}
