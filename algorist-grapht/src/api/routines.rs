pub(crate) mod basic {
    pub(crate) mod board {
        use std::{
            cmp::Ordering,
            error::Error,
            fmt::{Debug, Display, Formatter, Write},
            num::NonZeroIsize,
            ops::ControlFlow,
        };

        use algorist_grapht_macros::replace_fields;
        use thiserror::Error;

        use crate::api::{FieldsExt, GraphBackend};
        use crate::{
            api::{IdExt, VertexIterExt},
            backend::Graph,
        };

        #[derive(Debug, Error)]
        pub(crate) enum NormalizationError {
            #[error("allocation of output component ranges failed")]
            ComponentRangesAllocFailed,
        }

        pub(crate) fn normalize_board_size(
            n1: &mut isize,
            n2: &mut isize,
            n3: &mut isize,
            n4: &mut isize,
        ) -> Result<(Vec<isize>, usize), NormalizationError> {
            if *n1 <= 0 {
                let mut output = Vec::try_with_capacity(2)
                    .map_err(|_| NormalizationError::ComponentRangesAllocFailed)?;
                output.push(8);
                output.push(8);
                return Ok((output, 2));
            }
            let prior_components = [*n1, *n2, *n3];
            let (component_ranges, dims) = {
                let (component_ranges, dims) = [*n2, *n3, *n4]
                    .iter()
                    .enumerate()
                    .map(|(component_num, component)| (component_num + 1, component))
                    .try_fold(
                        {
                            let mut output = Vec::try_with_capacity(4)
                                .map_err(|_| NormalizationError::ComponentRangesAllocFailed)?;
                            output.push(*n1);

                            output
                        },
                        |mut total_components, (component_num, &component)| match component.cmp(&0)
                        {
                            Ordering::Less => ControlFlow::Break((
                                {
                                    let len = component.unsigned_abs();
                                    total_components.clear();
                                    if total_components.try_reserve_exact(len).is_ok() {
                                        prior_components
                                            .iter()
                                            .take(component_num)
                                            .cycle()
                                            .take(len)
                                            .copied()
                                            .collect_into(&mut total_components);

                                        Ok(total_components)
                                    } else {
                                        Err(total_components)
                                    }
                                },
                                component.unsigned_abs(),
                            )),
                            Ordering::Equal => ControlFlow::Break((
                                {
                                    total_components.clear();
                                    prior_components
                                        .iter()
                                        .take(component_num)
                                        .copied()
                                        .collect_into(&mut total_components);

                                    Ok(total_components)
                                },
                                component_num,
                            )),
                            Ordering::Greater => {
                                total_components.push(component);
                                ControlFlow::Continue(total_components)
                            }
                        },
                    )
                    .map_continue(|result| (Ok(result), 4))
                    .into_value();
                (
                    component_ranges.map_err(|_| NormalizationError::ComponentRangesAllocFailed)?,
                    dims,
                )
            };

            Ok((component_ranges, dims))
        }

        #[derive(Debug, Error)]
        pub(crate) enum BuildGraphError<G: GraphBackend> {
            #[error("")]
            ComponentSizesOutOfBounds,
            #[error("")]
            GraphCreationFailed(G::Error),
            #[error("")]
            AuxiliaryAllocFailed,
            #[error("")]
            FaultyStreamWrite,
        }

        #[cfg_attr(not(doc), replace_fields)]
        pub(crate) fn build_graph<
            S,
            G: GraphBackend<Vertex: IdExt<Id = S>>
                + for<'a> VertexIterExt<'a, G>
                + IdExt<Id = S>
                + FieldsExt<usize, 3>,
        >(
            nn: &[isize],
        ) -> Result<G, BuildGraphError<G>>
        where
            for<'a> &'a str: Into<S>,
        {
            let n = nn
                .iter()
                .try_fold(1_isize, |sum, &component| sum.checked_mul(component))
                .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?;
            let mut graph = G::new(n).map_err(|e| BuildGraphError::GraphCreationFailed(e))?;
            let mut name_state = {
                let mut output = Vec::try_with_capacity(nn.len())
                    .map_err(|_| BuildGraphError::AuxiliaryAllocFailed)?;
                (0..nn.len()).map(|_| 0_isize).collect_into(&mut output);

                output
            };
            let mut name = String::try_with_capacity(name_state.len())
                .map_err(|_| BuildGraphError::AuxiliaryAllocFailed)?;
            for vertex in graph.iter_mut() {
                vertex.set_id({
                    name = {
                        let mut output =
                            name_state.iter().try_fold(name, |mut name, component| {
                                write!(&mut name, "{component}.")
                                    .map_err(|_| BuildGraphError::FaultyStreamWrite)?;

                                Ok(name)
                            })?;
                        output.pop(); // Get rid of the last `.`.

                        output
                    };

                    name.as_str()
                });
                name.clear();
                for (component_num, component) in name_state.iter_mut().enumerate().rev() {
                    if *component + 1 == nn[component_num] {
                        *component = 0;
                        continue;
                    }
                    *component += 1;
                    break;
                }
            }

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
                    Self::GraphCreationFailed(e) => Display::fmt(e, f),
                }
            }
        }

        #[derive(Debug, Error)]
        pub(crate) enum BoardError<G: GraphBackend> {
            #[error("allocation of component size vector failed")]
            Normalization,
            #[error("failed to build graph: {0}")]
            GraphBuild(GraphBuildErrorSrc<G>),
            /// If you hit this error, some implementation detail hidden from
            /// the reach of library users has taken place.
            ///
            /// The error is fairly opaque and thus not meant to communicate
            /// anything but the fact that an unrecoverable error took place
            /// somewhere in the implementation.
            #[error(transparent)]
            Other(#[from] Box<dyn Error>),
        }

        impl<G: GraphBackend> From<NormalizationError> for BoardError<G> {
            fn from(value: NormalizationError) -> Self {
                match value {
                    NormalizationError::ComponentRangesAllocFailed => Self::Normalization,
                }
            }
        }

        impl<G: GraphBackend + Debug> From<BuildGraphError<G>> for BoardError<G>
        where
            for<'a> G: 'a,
        {
            fn from(value: BuildGraphError<G>) -> Self {
                match value {
                    BuildGraphError::ComponentSizesOutOfBounds => {
                        Self::GraphBuild(GraphBuildErrorSrc::ComponentSizesOutOfBounds)
                    }
                    BuildGraphError::GraphCreationFailed(e) => {
                        Self::GraphBuild(GraphBuildErrorSrc::GraphCreationFailed(e))
                    }
                    e => Self::Other({
                        let output: Box<dyn Error> = Box::new(e);

                        output
                    }),
                }
            }
        }

        #[cfg_attr(not(doc), replace_fields)]
        pub(crate) trait Board:
            GraphBackend<Vertex: IdExt<Id = <Self as Board>::Id>>
            + for<'a> VertexIterExt<'a, Self>
            + IdExt<Id = <Self as Board>::Id>
            + FieldsExt<usize, 3>
            + Debug
        where
            for<'a> &'a str: Into<<Self as Board>::Id>,
            for<'a> Self: 'a,
        {
            type Id;

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

        impl Board for Graph {
            type Id = String;
        }
    }

    fn simplex() {}

    fn subsets() {}

    fn perms() {}

    fn parts() {}

    fn binary() {}
}
