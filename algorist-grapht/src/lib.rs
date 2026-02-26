#![feature(
    allocator_api,
    try_with_capacity,
    control_flow_into_value,
    iter_collect_into
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
    use std::{error::Error, num::NonZeroIsize};

    use crate::backend::Graph;

    #[test]
    fn it_works() -> Result<(), Box<dyn Error>> {
        let mut graph = Graph::new(10)?;
        let _ = Graph::board(1, 1, 1, 1, unsafe { NonZeroIsize::new_unchecked(1) }, 1, 1);

        Ok(())
    }
}
