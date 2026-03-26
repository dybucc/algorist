use std::{
  alloc::AllocError,
  any::{self, Any, TypeId},
  borrow::{Borrow, BorrowMut},
  debug_assert_matches,
  fmt::{Display, Formatter},
  marker::PhantomData,
  num::NonZeroIsize,
  ptr,
  rc::Rc,
};

use num_traits::cast::AsPrimitive;
use thiserror::Error;

use crate::{
  api::{
    ArcAddExt,
    FieldsExt,
    GraphBackend,
    IdExt,
    VertexIterExt,
    routines::basic::board::{Board, BoardError},
  },
  fields::FieldBuilder,
};

#[derive(Debug)]
pub(crate) struct Arc {
  pub(crate) tip:    Option<Rc<Vertex>>,
  pub(crate) fields: FieldBuilder,
  pub(crate) id:     String,
}

impl PartialEq for Arc {
  fn eq(&self, other: &Self) -> bool {
    matches!((&self.tip, &other.tip), (Some(tip1), Some(tip2)) if Rc::ptr_eq(tip1, tip2))
  }
}

impl IdExt for Arc {
  type Id = String;

  fn get_id<T: ?Sized>(&self) -> &T
  where
    <Self as IdExt>::Id: Borrow<T>,
  {
    self.id.borrow()
  }

  fn set_id_with<T: Into<<Self as IdExt>::Id>>(
    &mut self,
    other_fn: impl FnOnce() -> T,
  ) {
    self.id = other_fn().into();
  }
}

#[derive(Debug)]
pub(crate) struct Vertex {
  pub(crate) arcs:   Vec<Rc<Arc>>,
  pub(crate) fields: FieldBuilder,
  pub(crate) id:     String,
}

impl IdExt for Vertex {
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

#[derive(Debug)]
pub(crate) struct Graph {
  pub(crate) vertices: Vec<Rc<Vertex>>,
  pub(crate) fields:   FieldBuilder,
  pub(crate) id:       String,
}

#[derive(Debug, Error)]
pub(crate) enum TryIterMutError {
  #[error("failed to allocate auxiliary memory")]
  AllocFailed,
  #[error("vertex with index {0} is not uniquely owned")]
  NonUniqueOwnersip(usize),
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

  pub(crate) fn try_iter_mut(
    &mut self,
  ) -> Result<TryIterMut<'_>, TryIterMutError> {
    let len = self.vertices.len();

    Ok(TryIterMut {
      container: self.vertices.iter_mut().enumerate().try_fold(
        Vec::try_with_capacity(len)
          .map_err(|_| TryIterMutError::AllocFailed)?,
        |mut container, (idx, ptr)| {
          container.push(
            &raw mut *Rc::get_mut(ptr)
              .ok_or(TryIterMutError::NonUniqueOwnersip(idx))?,
          );

          Ok(container)
        },
      )?,
      idx:       None,
      _marker:   PhantomData,
    })
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

pub(crate) struct IterMut<'a> {
  pub(crate) len:   usize,
  pub(crate) idx:   Option<usize>,
  pub(crate) graph: &'a mut Graph,
}

impl<'a> Iterator for IterMut<'a> {
  type Item = &'a mut Vertex;

  fn next(&mut self) -> Option<Self::Item> {
    match self.idx {
      | None => {
        if self.len == 0 {
          return None;
        }
        self.idx = Some(0);
      },
      | Some(ref mut idx) => {
        if *idx == self.len - 1 {
          return None;
        }
        *idx += 1;
      },
    }

    // SAFETY: the index is always `None` at this point, because the above
    // logic ensures that. The pointer is never `null` because of the
    // invariants held by `Rc`, and the lifetime is tied to that of the
    // underlying `graph`.
    self
      .graph
      .vertices
      .get_mut(unsafe { self.idx.unwrap_unchecked() })
      .map(|ptr| unsafe { Rc::as_ptr(ptr).cast_mut().as_mut_unchecked() })
  }
}

pub(crate) struct Iter<'a> {
  pub(crate) len:   usize,
  pub(crate) idx:   Option<usize>,
  pub(crate) graph: &'a Graph,
}

impl<'a> Iterator for Iter<'a> {
  type Item = &'a Vertex;

  fn next(&mut self) -> Option<Self::Item> {
    match self.idx {
      | None => {
        if self.len > 0 {
          return None;
        }
        self.idx = Some(0);
      },
      | Some(ref mut idx) => {
        if *idx == self.len - 1 {
          return None;
        }
        *idx += 1;
      },
    }

    // SAFETY: see the safety comment on the same method impl for `IterMut`.
    self
      .graph
      .vertices
      .get(unsafe { self.idx.unwrap_unchecked() })
      .map(|ptr| unsafe { Rc::as_ptr(ptr).as_ref_unchecked() })
  }
}

pub(crate) struct TryIterMut<'a> {
  pub(crate) container: Vec<*mut Vertex>,
  pub(crate) idx:       Option<usize>,
  pub(crate) _marker:   PhantomData<&'a mut Vertex>,
}

impl<'a> Iterator for TryIterMut<'a> {
  type Item = &'a mut Vertex;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(idx) = &mut self.idx {
      *idx += 1;
    } else {
      self.idx = Some(0);
    }

    unsafe {
      self
        .container
        .get(self.idx.unwrap_unchecked())
        .map(|ptr| ptr.as_mut_unchecked())
    }
  }
}

#[derive(Debug, Error)]
pub(crate) enum GraphCreationError {
  #[error("failed to allocate requested memory: allocation of {0} failed")]
  AllocError(AllocErrorSrc),
}

#[derive(Debug)]
pub(crate) enum AllocErrorSrc {
  ArenaAlloc,
  ItemInArena(ItemInArena),
}

impl Display for AllocErrorSrc {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      | Self::ArenaAlloc => write!(f, "arena blocks"),
      | Self::ItemInArena(item) => write!(f, "{} {}", item.0, item.1),
    }
  }
}

impl From<ItemInArena> for AllocErrorSrc {
  fn from(value: ItemInArena) -> Self { Self::ItemInArena(value) }
}

#[derive(Debug)]
pub(crate) struct ItemInArena(pub(crate) usize, pub(crate) ArenaItemType);

#[derive(Debug)]
pub(crate) enum ArenaItemType {
  Vert,
  Arc,
}

impl Display for ArenaItemType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    use ArenaItemType::{Arc, Vert};

    match self {
      | Vert => write!(f, "vertices"),
      | Arc => write!(f, "arcs"),
    }
  }
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
            ItemInArena(n, ArenaItemType::Vert),
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
      _ = unsafe {
        let one =
          Rc::as_ptr(self.vertices.get_unchecked(*src.borrow())).cast_mut();

        (
          (*one).arcs.try_reserve(1).map_err(|_| {
            ArcAddError::AuxiliaryAlloc(AllocFailureSrc::ArcCreation)
          })?,
          (*one).arcs.push(
            Arc {
              tip:    Rc::clone(self.vertices.get_unchecked(*dst.borrow()))
                .into(),
              fields: FieldBuilder::default(),
              id:     String::new(),
            }
            .into(),
          ),
        )
      },
    )
  }
}

#[derive(Error, Debug)]
pub(crate) enum FieldsExtError {
  #[error("auxiliary allocation failed: {0}")]
  AllocFailed(AllocFailureKind),
}

impl From<AllocFailureKind> for FieldsExtError {
  fn from(value: AllocFailureKind) -> Self { Self::AllocFailed(value) }
}

#[derive(Debug)]
pub(crate) enum AllocFailureKind {
  Type(&'static str),
  Bucket(&'static str),
  BucketKey(&'static str),
}

impl Display for AllocFailureKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      | Self::Type(ty) => write!(f, "new type allocation failed: {ty}"),
      | Self::Bucket(ty) =>
        write!(f, "bucket allocation failed for type: {ty}"),
      | Self::BucketKey(ty) =>
        write!(f, "allocation of container for bucket of types: `{ty}` failed"),
    }
  }
}

impl<T, const N: usize> FieldsExt<T, N> for Vertex
where
  for<'a> T: 'a,
{
  type Error = FieldsExtError;

  fn chfield_with<
    'a,
    Q: 'a,
    R: Into<T>,
    E: Into<<Self as FieldsExt<T, N>>::Error>,
  >(
    &mut self,
    mut producer: impl FnMut() -> Result<R, E>,
  ) -> Result<[&mut Q; N], <Self as FieldsExt<T, N>>::Error>
  where
    T: BorrowMut<Q> + 'a,
  {
    fn extract_n<'a, S: FieldsExt<T, N> + 'a, T, Q: 'a, const N: usize>(
      entry: &mut Vec<Box<dyn Any>>,
    ) -> [&'a mut Q; N]
    where
      for<'b> T: BorrowMut<Q> + 'b,
    {
      let mut output = [ptr::null_mut(); N];
      entry.iter_mut().enumerate().take(N).for_each(|(i, ty)| {
        // SAFETY: all elements `ty` in `entry` are of type `T` by virtue of
        // hashing from `T`'s `TypeId` to the bucket of values `ty` of type `T`.
        output[i] = unsafe { ty.downcast_unchecked_mut::<T>().borrow_mut() };
      });

      // SAFETY: the pointer actually points to the underlying value behind the
      // `Box<dyn Any>` of the hashmap `entry` is sourced from, so producing a
      // reference to it is sound.
      output.map(|ty| unsafe { ty.as_mut_unchecked() })
    }

    // This doesn't use the `Entry` API because that API uses calls to
    // allocation-wise fallible rouines that panic on failure.
    if let Some(entry) = self.fields.0.get_mut(&TypeId::of::<T>()) {
      Ok(extract_n::<Self, T, Q, N>((entry.len()..N).try_fold(
        {
          entry
            .try_reserve_exact(N)
            .map_err(|_| AllocFailureKind::Bucket(any::type_name::<T>()))?;

          entry
        },
        |entry, _| {
          entry.push({
            let out: Box<dyn Any> =
              Box::try_new(producer().map(Into::into).map_err(Into::into)?)
                .map_err(|_| AllocFailureKind::Type(any::type_name::<T>()))?;

            out
          });

          Ok::<_, FieldsExtError>(entry)
        },
      )?))
    } else {
      self
        .fields
        .0
        .try_reserve(1)
        .map_err(|_| AllocFailureKind::BucketKey(any::type_name::<T>()))?;
      self.fields.0.insert(
        TypeId::of::<T>(),
        (0..N).try_fold(
          Vec::try_with_capacity(N)
            .map_err(|_| AllocFailureKind::Bucket(any::type_name::<T>()))?,
          |mut entry, _| {
            entry.push({
              let out: Box<dyn Any> =
                Box::try_new(producer().map(Into::into).map_err(Into::into)?)
                  .map_err(|_| AllocFailureKind::Type(any::type_name::<T>()))?;

              out
            });

            Ok::<_, FieldsExtError>(entry)
          },
        )?,
      );

      // SAFETY: the key just got a bucket inserted above.
      Ok(extract_n::<Self, T, Q, N>(unsafe {
        self.fields.0.get_mut(&TypeId::of::<T>()).unwrap_unchecked()
      }))
    }
  }
}

impl Board for Graph {
  type ArcId = String;
  type GraphId = String;
  type VertexId = String;
}

pub(crate) mod cmds {}
