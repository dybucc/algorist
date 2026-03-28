use std::rc::Rc;

use crate::backend::{Graph, Vertex};

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

    // SAFETY: the index is always `None` at this point, because the above logic
    // ensures that. The pointer is never `null` because of the invariants held
    // by `Rc`, and the lifetime is tied to that of the underlying `graph`.
    self
      .graph
      .vertices
      .get_mut(unsafe { self.idx.unwrap_unchecked() })
      .map(|ptr| unsafe { Rc::as_ptr(ptr).cast_mut().as_mut_unchecked() })
  }
}
