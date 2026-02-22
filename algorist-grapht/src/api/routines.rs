use crate::api::GraphBackend;
use thiserror::Error;

pub(crate) mod basic {
    #![expect(
        clippy::wildcard_imports,
        reason = "This module is purposefully structured this way. There's some imports that all \
                 submodules need. The lint expectation is unfulfilled, but without it clippy still \
                 complains."
    )]

    use super::*;

    pub(crate) mod board {
        use std::{
            cmp::Ordering,
            collections::TryReserveError,
            fmt::{Display, Formatter},
            num::NonZeroIsize,
            ops::ControlFlow,
        };

        use super::*;

        #[derive(Error, Debug)]
        pub(crate) enum NormalizationError {
            #[error("allocation of component size vector failed")]
            AllocFailed,
        }

        impl From<TryReserveError> for NormalizationError {
            fn from(_: TryReserveError) -> Self {
                NormalizationError::AllocFailed
            }
        }

        pub(crate) fn normalize_board_size(
            n1: &mut isize,
            n2: &mut isize,
            n3: &mut isize,
            n4: &mut isize,
        ) -> Result<(Vec<isize>, usize), NormalizationError> {
            if *n1 <= 0 {
                let mut output = Vec::try_with_capacity(2)?;
                output.push(8);
                output.push(8);

                return Ok((output, 2));
            }

            let prior_components = [*n1, *n2, *n3];
            Ok([*n2, *n3, *n4]
                .iter()
                .enumerate()
                .map(|(component_num, component)| (component_num + 1, component))
                .try_fold(
                    {
                        let mut output = Vec::try_with_capacity(1)?;
                        output.push(*n1);

                        output
                    },
                    |mut total_components, (component_num, &component)| match component.cmp(&0) {
                        Ordering::Less => ControlFlow::Break((
                            prior_components
                                .iter()
                                .take(component_num)
                                .cycle()
                                .take(component.unsigned_abs())
                                .copied()
                                .collect::<Vec<_>>(),
                            component.unsigned_abs(),
                        )),
                        Ordering::Equal => ControlFlow::Break((
                            prior_components
                                .iter()
                                .take(component_num)
                                .copied()
                                .collect::<Vec<_>>(),
                            component_num,
                        )),
                        Ordering::Greater => {
                            total_components.push(component);
                            ControlFlow::Continue(total_components)
                        }
                    },
                )
                .map_continue(|result| (result, 4))
                .into_value())
        }

        #[derive(Debug, Error)]
        pub(crate) enum BuildGraphError<G: GraphBackend> {
            #[error("value range for component is disproportionately large")]
            ComponentSizesOutOfBounds,
            #[error("failed to create graph")]
            GraphCreationFailed(G::Error),
        }

        pub(crate) fn build_graph<G: GraphBackend>(
            dims: usize,
            nn: &[isize],
        ) -> Result<G, BuildGraphError<G>> {
            let n = nn
                .iter()
                .try_fold(1_isize, |accum, &component| accum.checked_mul(component))
                .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?;
            let graph = G::new(n).map_err(|e| BuildGraphError::GraphCreationFailed(e))?;

            todo!()
        }

        pub(crate) fn fill_arcs() {}

        #[derive(Debug)]
        pub(crate) enum GraphBuildErrorSrc<G: GraphBackend> {
            ComponentSizesOutOfBounds,
            GraphCreationFailed(G::Error),
        }

        impl<G: GraphBackend> Display for GraphBuildErrorSrc<G> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::ComponentSizesOutOfBounds => {
                        write!(f, "value range for component is disproportionately large")
                    }
                    Self::GraphCreationFailed(e) => e.fmt(f),
                }
            }
        }

        #[derive(Debug, Error)]
        pub(crate) enum BoardError<G: GraphBackend> {
            #[error("allocation of component size vector failed")]
            NormalizationFailed,
            #[error("failed to build graph: {0}")]
            GraphBuildFailed(GraphBuildErrorSrc<G>),
        }

        impl<G: GraphBackend> From<NormalizationError> for BoardError<G> {
            fn from(value: NormalizationError) -> Self {
                match value {
                    NormalizationError::AllocFailed => Self::NormalizationFailed,
                }
            }
        }

        impl<G: GraphBackend> From<BuildGraphError<G>> for BoardError<G> {
            fn from(value: BuildGraphError<G>) -> Self {
                match value {
                    BuildGraphError::ComponentSizesOutOfBounds => {
                        Self::GraphBuildFailed(GraphBuildErrorSrc::ComponentSizesOutOfBounds)
                    }
                    BuildGraphError::GraphCreationFailed(e) => {
                        Self::GraphBuildFailed(GraphBuildErrorSrc::GraphCreationFailed(e))
                    }
                }
            }
        }

        pub(crate) trait Board: GraphBackend {
            fn board(
                mut n1: isize,
                mut n2: isize,
                mut n3: isize,
                mut n4: isize,
                piece: NonZeroIsize,
                wrap: isize,
                directed: isize,
            ) -> Result<Self, BoardError<Self>> {
                let (nn, d) = normalize_board_size(&mut n1, &mut n2, &mut n3, &mut n4)?;
                let graph: Self = build_graph(d, &nn)?;
                fill_arcs();

                Ok(graph)
            }

            fn complete() {}
            fn transitive() {}
            fn empty() {}
            fn circuit() {}
            fn cycle() {}
        }

        // TODO: expose a type that takes a generic parameter that implements
        // `GraphBackend` and makes more ergonomic calling into the trait's
        // methods.
    }

    fn simplex() {}

    fn subsets() {}

    fn perms() {}

    fn parts() {}

    fn binary() {}
}
