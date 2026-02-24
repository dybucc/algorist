pub(crate) mod basic {
    pub(crate) mod board {
        use std::{
            cmp::Ordering,
            collections::TryReserveError,
            fmt::{Display, Formatter},
            num::NonZeroIsize,
            ops::ControlFlow,
        };

        use thiserror::Error;

        use crate::api::GraphBackend;
        use crate::{
            api::{IdExt, VertexIterExt},
            backend::Graph,
        };

        #[derive(Debug)]
        pub(crate) enum NormalizationError {
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

        #[derive(Debug)]
        pub(crate) enum BuildGraphError<G: GraphBackend> {
            ComponentSizesOutOfBounds,
            GraphCreationFailed(G::Error),
            AuxiliaryAllocFailed(Context),
        }

        impl<G: GraphBackend> From<TryReserveError> for BuildGraphError<G> {
            fn from(value: TryReserveError) -> Self {
                Self::AuxiliaryAllocFailed(Context)
            }
        }

        pub(crate) fn build_graph<
            S,
            G: GraphBackend<Vertex: IdExt<Id = S>> + for<'a> VertexIterExt<'a, G> + IdExt<Id = S>,
        >(
            nn: &[isize],
        ) -> Result<G, BuildGraphError<G>>
        where
            for<'a> &'a str: Into<S>,
        {
            let n = nn
                .iter()
                .try_fold(1_isize, |accum, &component| accum.checked_mul(component))
                .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?;
            let mut graph = G::new(n).map_err(|e| BuildGraphError::GraphCreationFailed(e))?;
            let mut name_state = Vec::try_with_capacity(nn.len())?;
            (0..nn.len()).map(|_| 0_usize).collect_into(&mut name_state);
            for vertex in graph.iter_mut() {}

            Ok(graph)
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

        pub(crate) enum Context {
            NameAllocation,
        }

        #[derive(Debug, Error)]
        pub(crate) enum BoardError<G: GraphBackend> {
            #[error("allocation of component size vector failed")]
            NormalizationFailed,
            #[error("failed to build graph: {0}")]
            GraphBuildFailed(GraphBuildErrorSrc<G>),
            #[error("failed to allocate auxiliary memory during {0}")]
            AuxiliaryAllocFailed(Context),
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
                    BuildGraphError::AuxiliaryAllocFailed => Self::AuxiliaryAllocFailed,
                }
            }
        }

        pub(crate) trait Board<S = String>:
            GraphBackend<Vertex: IdExt<Id = S>>
            + for<'a> VertexIterExt<'a, Self>
            + IdExt<Id = S>
        where
            for<'a> &'a str: Into<S>,
        {
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
                let graph: Self = build_graph(&nn)?;
                fill_arcs();

                Ok(graph)
            }

            fn complete() {}
            fn transitive() {}
            fn empty() {}
            fn circuit() {}
            fn cycle() {}
        }

        impl Board for Graph {}
    }

    fn simplex() {}

    fn subsets() {}

    fn perms() {}

    fn parts() {}

    fn binary() {}
}
