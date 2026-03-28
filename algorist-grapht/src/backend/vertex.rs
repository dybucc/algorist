use std::{
  any::{self, Any, TypeId},
  borrow::{Borrow, BorrowMut, Cow},
  ptr,
  rc::Rc,
};

use thiserror::Error;

use crate::{
  api::{FieldsExt, IdExt},
  backend::Arc,
  fields::FieldBuilder,
};

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

    macro_rules! new_entry {
      ($entry:expr) => {{
        Ok::<_, FieldsExtError>(
          (
            $entry.push({
              let out: Box<dyn Any> =
                Box::try_new(producer().map(Into::into).map_err(Into::into)?)
                  .map_err(|_| {
                  FieldsExtError(AllocFailureKind::Type(
                    any::type_name::<T>().into(),
                  ))
                })?;

              out
            }),
            $entry,
          )
            .1,
        )
      }};
    }

    // This doesn't use the `Entry` API because that API uses calls to
    // allocation-wise fallible rouines that panic on failure.
    if let Some(entry) = self.fields.0.get_mut(&TypeId::of::<T>()) {
      Ok(extract_n::<Self, T, Q, N>((entry.len()..N).try_fold(
        {
          entry.try_reserve_exact(N).map_err(|_| {
            FieldsExtError(AllocFailureKind::Bucket(
              any::type_name::<T>().into(),
            ))
          })?;

          entry
        },
        |entry, _| new_entry!(entry),
      )?))
    } else {
      self.fields.0.try_reserve(1).map_err(|_| {
        FieldsExtError(AllocFailureKind::BucketKey(
          any::type_name::<T>().into(),
        ))
      })?;
      self.fields.0.insert(
        TypeId::of::<T>(),
        (0..N).try_fold(
          Vec::try_with_capacity(N).map_err(|_| {
            FieldsExtError(AllocFailureKind::Bucket(
              any::type_name::<T>().into(),
            ))
          })?,
          |mut entry, _| new_entry!(entry),
        )?,
      );

      // SAFETY: the key just got a bucket inserted above.
      Ok(extract_n::<Self, T, Q, N>(unsafe {
        self.fields.0.get_mut(&TypeId::of::<T>()).unwrap_unchecked()
      }))
    }
  }
}

#[derive(Error, Debug)]
#[error(
  "auxiliary allocation failed: {}",
  match .0 {
    | AllocFailureKind::Type(ty) =>
      format!("new type allocation failed: {ty}"),
    | AllocFailureKind::Bucket(ty) =>
      format!("bucket allocation failed for type: {ty}"),
    | AllocFailureKind::BucketKey(ty) =>
      format!("allocation of container for bucket of types: `{ty}` failed"),
  }
)]
pub(crate) struct FieldsExtError(AllocFailureKind);

#[derive(Debug)]
pub(crate) enum AllocFailureKind {
  Type(Cow<'static, str>),
  Bucket(Cow<'static, str>),
  BucketKey(Cow<'static, str>),
}
