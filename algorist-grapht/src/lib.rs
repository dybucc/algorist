#![feature(
    allocator_api,
    try_with_capacity,
    control_flow_into_value,
    iter_collect_into,
    downcast_unchecked
)]
#![expect(dead_code, reason = "The crate is a WIP.")]

pub mod api;
pub mod backend;
pub mod fields;
mod private {
    pub(crate) trait Sealed {}
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fmt::Write};

    use crate::{
        api::{FieldsExt, GraphBackend},
        backend::Graph,
    };

    #[test]
    fn it_works() -> Result<(), Box<dyn Error>> {
        let mut graph = Graph::new(10)?;
        for vertex in &mut graph {
            eprintln!("initial state");
            let [a, b, c]: [&mut String; _] =
                <<Graph as GraphBackend>::Vertex as FieldsExt<String, 3>>::chfield(vertex).unwrap();
            eprintln!("{a}\n{b}\n{c}\n---");
        }
        for vertex in &mut graph {
            eprintln!("modification");
            let [a, b, c]: [&mut String; _] =
                <<Graph as GraphBackend>::Vertex as FieldsExt<String, 3>>::chfield(vertex).unwrap();
            a.reserve_exact("Something".len());
            write!(a, "Something").unwrap();
            b.reserve_exact("Something".len());
            write!(b, "Something").unwrap();
            c.reserve_exact("Something".len());
            write!(c, "Something").unwrap();
        }
        for vertex in &mut graph {
            eprintln!("final state");
            let [a, b, c]: [&mut String; _] =
                <<Graph as GraphBackend>::Vertex as FieldsExt<String, 3>>::chfield(vertex).unwrap();
            eprintln!("{a}\n{b}\n{c}\n---");
        }

        Ok(())
    }
}
