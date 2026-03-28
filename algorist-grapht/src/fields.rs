use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

// TODO: see if it's worth recovering the `FieldBuilder` API, or a wrapper type
// should do.

#[derive(Debug, Default)]
pub(crate) struct FieldBuilder(pub(crate) HashMap<TypeId, Vec<Box<dyn Any>>>);
