#![feature(
  allocator_api, try_with_capacity, control_flow_into_value, iter_collect_into,
  downcast_unchecked
)]
#![expect(dead_code, reason = "The crate is a WIP.")]

pub(crate) mod api;
pub(crate) mod backend;
pub(crate) mod fields;
pub(crate) mod macros;
pub(crate) mod private {
  pub(crate) trait Sealed {}
}

#[cfg(test)]
mod tests {
  use std::{error::Error, fmt::Write as _};

  use crate::{backend::Graph, fields_of};

  #[test]
  fn it_works() -> Result<(), Box<dyn Error>> {
    let mut graph = Graph::new(10)?;
    for vertex in &mut graph {
      eprintln!("initial state");
      let [a, b, c]: [&mut String; _] =
        fields_of!(String; 3 => v in Graph: vertex).unwrap();
      eprintln!("{a}\n{b}\n{c}\n---");
    }
    for vertex in &mut graph {
      eprintln!("modification");
      let [a, b, c]: [&mut String; _] =
        fields_of!(String; 3 => v in Graph: vertex).unwrap();
      a.reserve_exact("Something".len());
      b.reserve_exact("Something".len());
      c.reserve_exact("Something".len());
      write!(a, "Something").unwrap();
      write!(b, "Something").unwrap();
      write!(c, "Something").unwrap();
    }
    for vertex in &mut graph {
      eprintln!("final state");
      let [a, b, c]: [&mut String; _] =
        fields_of!(String; 3 => v in Graph: vertex).unwrap();
      eprintln!("{a}\n{b}\n{c}\n---");
    }

    Ok(())
  }
}
