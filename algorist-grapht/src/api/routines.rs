pub(crate) mod basic {
    pub(crate) mod board {
        use std::{
            cmp::Ordering,
            collections::TryReserveError,
            error::Error,
            fmt::{Debug, Display, Formatter, Write as _},
            hint::unreachable_unchecked,
            num::NonZeroIsize,
            ops::ControlFlow,
        };

        use thiserror::Error;

        use crate::api::{FieldsExt, GraphBackend, IdExt, VertexIterExt};

        #[derive(Debug, Error)]
        pub(crate) enum NormalizationError {
            #[error("allocation of output component ranges failed")]
            ComponentRangesAllocFailed,
        }

        impl From<TryReserveError> for NormalizationError {
            fn from(_: TryReserveError) -> Self {
                Self::ComponentRangesAllocFailed
            }
        }

        pub(crate) fn normalize_board_size(
            n1: &mut isize,
            n2: &mut isize,
            n3: &mut isize,
            n4: &mut isize,
        ) -> Result<Vec<usize>, NormalizationError> {
            Ok([*n1, *n2, *n3, *n4]
                .iter()
                .enumerate()
                .try_fold(
                    Vec::try_with_capacity(4)?,
                    |mut components, (component_num, &component)| match component.cmp(&0) {
                        Ordering::Less | Ordering::Equal if component_num == 0 => {
                            ControlFlow::Break(Ok((0..2).fold(components, |mut output, _| {
                                output.push(8);

                                output
                            })))
                        }
                        Ordering::Less => ControlFlow::Break({
                            components.clear();

                            components
                                .try_reserve_exact(component.unsigned_abs())
                                .map(|()| {
                                    [*n1, *n2, *n3]
                                        .into_iter()
                                        .take(component_num)
                                        .cycle()
                                        .take(component.unsigned_abs())
                                        .map(isize::cast_unsigned)
                                        .collect_into(&mut components);

                                    components
                                })
                        }),
                        Ordering::Equal => ControlFlow::Break(Ok(components)),
                        Ordering::Greater => {
                            components.push(component.cast_unsigned());

                            ControlFlow::Continue(components)
                        }
                    },
                )
                .map_continue(Ok)
                .into_value()?)
        }

        #[derive(Debug, Error)]
        pub(crate) enum BuildGraphError {
            #[error("input component sizes produce a larger-than-signed machine word vertex count")]
            ComponentSizesOutOfBounds,
            #[error(transparent)]
            GraphCreationFailed(Box<dyn Error>),
            #[error("auxiliary heap allocation failed")]
            AuxiliaryAllocFailed,
            #[error("writing onto the name stream for vertex ids failed")]
            FaultyStreamWrite,
            #[error(transparent)]
            WrongFieldAccess(Box<dyn Error>),
        }

        impl From<TryReserveError> for BuildGraphError {
            fn from(_: TryReserveError) -> Self {
                Self::AuxiliaryAllocFailed
            }
        }

        pub(crate) fn build_graph<
            GId,
            VId,
            G: GraphBackend<Vertex: IdExt<Id = GId> + FieldsExt<usize, 3>>
                + for<'a> VertexIterExt<'a, G>
                + IdExt<Id = VId>,
        >(
            nn: &[usize],
        ) -> Result<G, BuildGraphError>
        where
            for<'a> &'a str: Into<GId> + Into<VId>,
            for<'a> <<G as GraphBackend>::Vertex as FieldsExt<usize, 3>>::Error: 'a,
            for<'a> <G as GraphBackend>::Error: 'a,
        {
            let (mut name_state, mut graph) = (
                (0..nn.len()).fold(Vec::try_with_capacity(nn.len())?, |mut output, _| {
                    output.push(0);

                    output
                }),
                G::new(
                    nn.iter()
                        .try_fold(1_usize, |sum, &component| sum.checked_mul(component))
                        .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?,
                )
                .map_err(|e| {
                    let inp: Box<dyn Error> = Box::new(e);

                    BuildGraphError::GraphCreationFailed(inp)
                })?,
            );
            graph.iter_mut().try_fold(
                // Must account for both the number of digits of the last
                // vertex's coordinate, as well as the separation dots.
                String::try_with_capacity(
                    nn.iter()
                        .map(|&component_range| {
                            // `!(c <= 0)` should hold for any component `c` in
                            // `nn` once `normalize_board_size()` is done.
                            debug_assert_ne!(component_range, 0);

                            component_range.ilog10() as usize
                        })
                        .sum::<usize>()
                        + nn.len(),
                )?,
                |mut name, vertex| {
                    name = {
                        let mut output = name_state.iter().enumerate().try_fold(
                            name,
                            |mut name, (idx, component)| {
                                write!(&mut name, "{component}.")
                                    .map_err(|_| BuildGraphError::FaultyStreamWrite)?;
                                (..3_usize).contains(&idx).then_some(()).iter().try_fold(
                                    (),
                                    |(), result| {
                                        let [x, y, z] = <<G as GraphBackend>::Vertex as FieldsExt<
                                        usize,
                                        3,
                                    >>::chfield(
                                        vertex
                                    )
                                    .map_err(|e| {
                                        let inp: Box<dyn Error> = Box::new(e);

                                        BuildGraphError::WrongFieldAccess(inp)
                                    })?;
                                        match idx {
                                            0 => *x = *component,
                                            1 => *y = *component,
                                            2 => *z = *component,
                                            // SAFETY: `idx` only ever takes on the
                                            // values in the range `0..3` if this
                                            // execution branch runs.
                                            _ => unsafe { unreachable_unchecked() },
                                        }

                                        Ok::<_, BuildGraphError>(())
                                    },
                                )?;

                                Ok::<_, BuildGraphError>(name)
                            },
                        )?;
                        output.pop(); // Get rid of the last `.`.

                        output
                    };
                    vertex.set_id(&*name);
                    name.clear();
                    let _ = name_state.iter_mut().enumerate().rev().try_fold(
                        (),
                        |(), (component_num, component)| {
                            if *component + 1 == nn[component_num] {
                                ControlFlow::Continue(())
                            } else {
                                *component += 1;

                                ControlFlow::Break(())
                            }
                        },
                    );

                    Ok::<_, BuildGraphError>(name)
                },
            )?;

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
            /// the reach of library users has failed.
            ///
            /// The error is fairly opaque and thus not meant to communicate
            /// anything but the fact that an unrecoverable error took place,
            /// and its meaning is not exposed in the public API.
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

        impl<G: GraphBackend + Debug> From<BuildGraphError> for BoardError<G>
        where
            for<'a> G: 'a,
        {
            fn from(value: BuildGraphError) -> Self {
                match value {
                    BuildGraphError::ComponentSizesOutOfBounds => {
                        Self::GraphBuild(GraphBuildErrorSrc::ComponentSizesOutOfBounds)
                    }
                    BuildGraphError::GraphCreationFailed(e) => {
                        Self::GraphBuild(GraphBuildErrorSrc::GraphCreationFailed(unsafe {
                            *e.downcast().unwrap_unchecked()
                        }))
                    }
                    e => Self::Other({
                        let output: Box<dyn Error> = Box::new(e);

                        output
                    }),
                }
            }
        }

        pub(crate) trait Board:
            GraphBackend<Vertex: IdExt<Id = <Self as Board>::VertexId> + FieldsExt<usize, 3>>
            + for<'a> VertexIterExt<'a, Self>
            + IdExt<Id = <Self as Board>::GraphId>
            + Debug
        where
            for<'a> &'a str: Into<<Self as Board>::GraphId>
                + Into<<Self as Board>::VertexId>
                + Into<<Self as Board>::ArcId>,
            for<'a> Self: 'a,
        {
            type GraphId;
            type VertexId;
            type ArcId;

            fn board(
                mut n1: isize,
                mut n2: isize,
                mut n3: isize,
                mut n4: isize,
                piece: NonZeroIsize,
                wrap: isize,
                directed: isize,
            ) -> Result<Self, BoardError<Self>> {
                let nn = normalize_board_size(&mut n1, &mut n2, &mut n3, &mut n4)?;
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
    }

    fn simplex() {}

    fn subsets() {}

    fn perms() {}

    fn parts() {}

    fn binary() {}
}
