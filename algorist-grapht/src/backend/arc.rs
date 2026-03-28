use std::{borrow::Borrow, rc::Rc};

use crate::{api::IdExt, backend::Vertex, fields::FieldBuilder};

#[derive(Debug)]
pub(crate) struct Arc {
  pub(crate) tip:    Option<Rc<Vertex>>,
  pub(crate) fields: FieldBuilder,
  pub(crate) id:     String,
}

impl PartialEq for Arc {
  fn eq(&self, other: &Self) -> bool {
    matches!(
      (&self.tip, &other.tip),
      (Some(tip1), Some(tip2)) if Rc::ptr_eq(tip1, tip2)
    )
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
