use std::rc::Rc;

use crate::backend::{Graph, Vertex};

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
