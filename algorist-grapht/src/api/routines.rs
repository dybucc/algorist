pub(crate) mod basic {
    pub(crate) mod board {
        use std::{
            cmp::Ordering,
            collections::TryReserveError,
            error::Error,
            fmt::{self, Debug, Display, Formatter, Write as _},
            hint,
            num::NonZeroIsize,
            ops::ControlFlow,
        };

        use thiserror::Error;

        use crate::{
            api::{FieldsExt, GraphBackend, IdExt, VertexIterExt},
            fields_of,
        };

        #[derive(Debug, Error)]
        pub(crate) enum NormalizationError {
            #[error("allocation of output component ranges failed")]
            ComponentRangesAllocFailed,
        }

        impl From<TryReserveError> for NormalizationError {
            fn from(_: TryReserveError) -> Self { Self::ComponentRangesAllocFailed }
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
                        | Ordering::Less | Ordering::Equal if component_num == 0 =>
                            ControlFlow::Break(Ok((0..2).fold(components, |mut output, _| {
                                output.push(8);

                                output
                            }))),
                        | Ordering::Less => ControlFlow::Break({
                            components.clear();

                            components.try_reserve_exact(component.unsigned_abs()).map(|()| {
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
                        | Ordering::Equal => ControlFlow::Break(Ok(components)),
                        | Ordering::Greater => ControlFlow::Continue({
                            components.push(component.cast_unsigned());

                            components
                        }),
                    },
                )
                .map_continue(Ok)
                .into_value()?)
        }

        #[derive(Debug, Error)]
        pub(crate) enum BuildGraphError {
            #[error("input component sizes produce a larger-than-signed machine word vertex count")]
            ComponentSizesOutOfBounds,
            #[error("failed to create graph: {0}")]
            GraphCreationFailed(#[source] Box<dyn Error>),
            #[error("auxiliary heap allocation failed")]
            AuxiliaryAllocFailed,
            #[error("writing onto the name stream for vertex ids failed")]
            FaultyStreamWrite,
            #[error(transparent)]
            WrongFieldAccess(Box<dyn Error>),
        }

        impl From<TryReserveError> for BuildGraphError {
            fn from(_: TryReserveError) -> Self { Self::AuxiliaryAllocFailed }
        }

        pub(crate) fn build_graph<
            GId,
            VId,
            G: GraphBackend<Vertex: IdExt<Id = GId> + FieldsExt<usize, 3>>
                + for<'a> VertexIterExt<'a, G>
                + IdExt<Id = VId>,
        >(
            component_range: &[usize],
        ) -> Result<G, BuildGraphError>
        where
            for<'a> &'a str: Into<GId> + Into<VId>,
            for<'a> <<G as GraphBackend>::Vertex as FieldsExt<usize, 3>>::Error: 'a,
            for<'a> <G as GraphBackend>::Error: 'a,
        {
            let (mut name_state, mut graph) = (
                (0..component_range.len()).fold(
                    Vec::try_with_capacity(component_range.len())?,
                    |mut output, _| {
                        output.push(0);

                        output
                    },
                ),
                G::new(
                    component_range
                        .iter()
                        .try_fold(1_usize, |sum, &component| sum.checked_mul(component))
                        .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?,
                )
                .map_err(|e| {
                    let ext: Box<dyn Error> = Box::new(e);

                    BuildGraphError::GraphCreationFailed(ext)
                })?,
            );
            graph.iter_mut().try_fold(
                // Must account for both the number of digits of the last
                // vertex's coordinate, as well as the separation dots.
                String::try_with_capacity(
                    component_range
                        .iter()
                        .map(|&component_range| {
                            // `c > 0` should hold for any component `c` in `nn`
                            // after running `normalize_board_size()`.
                            debug_assert_ne!(component_range, 0);

                            // +1 because `ilog10()` rounds down.
                            component_range.ilog10() as usize + 1
                        })
                        .sum::<usize>()
                        + component_range.len(),
                )?,
                |mut name, vertex| {
                    name = {
                        let mut name = name_state.iter().enumerate().try_fold(
                            name,
                            |mut name, (idx, component)| {
                                write!(&mut name, ".{component}")
                                    .map_err(|_| BuildGraphError::FaultyStreamWrite)?;
                                // TODO: the original GraphBase mentions that
                                // the first three components are saved in
                                // integer utility fields, but in theory,
                                // indices `0..3` point at the "last" three
                                // components, because `name_state` iterates
                                // from left to right in the following sequence:
                                // (a, b, ..., alpha, beta). The true "first"
                                // three components would be the last three in
                                // this iterator, but for now, we're only
                                // reproducing the same behavior as the one in
                                // the original GraphBase.
                                (..3).contains(&idx).then_some(()).iter().try_for_each(|()| {
                                    let [x, y, z] = fields_of!(usize; 3 => v in G: vertex)
                                        .map_err(|e| {
                                            let ext: Box<dyn Error> = Box::new(e);

                                            BuildGraphError::WrongFieldAccess(ext)
                                        })?;
                                    match idx {
                                        | 0 => *x = *component,
                                        | 1 => *y = *component,
                                        | 2 => *z = *component,
                                        // SAFETY: here `idx` only ever takes on
                                        // values in the range `0..3`.
                                        | _ => unsafe { hint::unreachable_unchecked() },
                                    }

                                    Ok::<_, BuildGraphError>(())
                                })?;

                                Ok::<_, BuildGraphError>(name)
                            },
                        )?;
                        name.pop(); // Get rid of the last `.`.

                        name
                    };
                    vertex.set_id(name.as_str());
                    name.clear();
                    name_state
                        .iter_mut()
                        .enumerate()
                        .rev()
                        .try_for_each(|(component_num, component)| {
                            if *component + 1 == component_range[component_num] {
                                ControlFlow::Continue(())
                            } else {
                                *component += 1;

                                ControlFlow::Break(())
                            }
                        })
                        .into_value();

                    Ok::<_, BuildGraphError>(name)
                },
            )?;

            Ok(graph)
        }

        #[derive(Debug, Error)]
        pub(crate) enum NamingError {
            #[error("auxiliary allocation failed: {0}")]
            AllocFailed(#[source] Box<dyn Error>),
            #[error("failed to write to naming string: {0}")]
            FaultyStreamWrite(#[source] Box<dyn Error>),
        }

        pub(crate) fn name_graph<GId, G: GraphBackend + IdExt<Id = GId>>(
            graph: &mut G,
            params: &[isize],
            directed: bool,
        ) -> Result<(), NamingError>
        where
            for<'a> &'a str: Into<GId>,
        {
            #![expect(clippy::unit_arg, reason = "Beauty comes at a cost.")]

            let mut graph_name = String::try_with_capacity(
                "board()".len()
                    + params
                        .iter()
                        .map(|param| {
                            if let Some(digits) = param.checked_ilog10() {
                                // +1 because `ilog10()` rounds down.
                                digits as usize + 1
                            } else {
                                let digits = param.unsigned_abs().ilog10() as usize;

                                // +2 if negative to account for `ilog10()`
                                // rounding down and the sign; Otherwise (i.e.
                                // 0,) account only for rounding down.
                                if param.is_negative() { digits + 2 } else { digits + 1 }
                            }
                        })
                        .sum::<usize>()
                    + 1 // Accounts for the `directed` parameter.
                    + params.len(), // Accounts for commas between parameters.
            )
            .map_err(|e| {
                let ext: Box<dyn Error> = Box::new(e);

                NamingError::AllocFailed(ext)
            })?;

            macro_rules! write_err {
                ($buf:expr, $($args:expr),+) => {{
                    write!($buf, $($args),+).map_err(|e| {
                        let ext: Box<dyn Error> = Box::new(e);

                        NamingError::FaultyStreamWrite(ext)
                    })?
                }};
            }

            write_err!(graph_name, "board(");
            params.iter().try_for_each(|param| Ok(write_err!(graph_name, "{param},")))?;
            write_err!(graph_name, "{}", if directed { "1" } else { "0" });
            write_err!(graph_name, ")");
            graph.set_id(graph_name.as_str());

            Ok(())
        }

        #[derive(Debug, Error)]
        pub(crate) enum InitStateError {
            #[error("auxilary allocation failed")]
            AuxliaryAllocFailed,
        }

        type InitState = (Vec<bool>, Vec<usize>, Vec<usize>);

        pub(crate) fn init_state(piece: isize, wrap: isize) -> Result<InitState, InitStateError> {
            let wrap = todo!();

            todo!()
        }

        #[derive(Debug, Error)]
        pub(crate) enum FillArcsError {
            #[error(transparent)]
            Other(Box<dyn Error>),
        }

        impl From<InitStateError> for FillArcsError {
            fn from(value: InitStateError) -> Self {
                match value {
                    | e => Self::Other({
                        let ext: Box<dyn Error> = Box::new(e);

                        ext
                    }),
                }
            }
        }

        pub(crate) fn fill_arcs<G: GraphBackend>(
            graph: &mut G,
            piece: isize,
            wrap: isize,
            directed: bool,
        ) -> Result<(), FillArcsError> {
            let (wr, del, sig) = init_state(piece, wrap)?;

            Ok(())
        }

        #[derive(Debug)]
        pub(crate) enum GraphBuildErrorKind {
            ComponentSizesOutOfBounds,
            GraphCreationFailed(Box<dyn Error>),
        }

        impl Display for GraphBuildErrorKind {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    | Self::ComponentSizesOutOfBounds =>
                        write!(f, "value range for component is disproportionately large"),
                    | Self::GraphCreationFailed(e) => Display::fmt(e, f),
                }
            }
        }

        #[derive(Debug, Error)]
        pub(crate) enum BoardError {
            #[error("failed to normalize component range sizes for d-dimensional adjacency matrix")]
            Normalization,
            #[error("failed to build graph: {0}")]
            GraphBuild(GraphBuildErrorKind),
            /// If you hit this error, some implementation detail hidden from
            /// the reach of library users has failed.
            ///
            /// The error is fairly opaque and thus not meant to communicate
            /// anything but the fact that an unrecoverable error took place,
            /// and its meaning is not exposed in the public API.
            #[error(transparent)]
            Other(#[from] Box<dyn Error>),
        }

        impl From<GraphBuildErrorKind> for BoardError {
            fn from(value: GraphBuildErrorKind) -> Self { Self::GraphBuild(value) }
        }

        impl From<NormalizationError> for BoardError {
            fn from(value: NormalizationError) -> Self {
                match value {
                    | NormalizationError::ComponentRangesAllocFailed => Self::Normalization,
                }
            }
        }

        impl From<BuildGraphError> for BoardError {
            fn from(value: BuildGraphError) -> Self {
                match value {
                    | BuildGraphError::ComponentSizesOutOfBounds =>
                        GraphBuildErrorKind::ComponentSizesOutOfBounds.into(),
                    | BuildGraphError::GraphCreationFailed(e) =>
                        GraphBuildErrorKind::GraphCreationFailed(e).into(),
                    | e => {
                        let output: Box<dyn Error> = Box::new(e);

                        output.into()
                    },
                }
            }
        }

        impl From<NamingError> for BoardError {
            fn from(value: NamingError) -> Self {
                match value {
                    | NamingError::AllocFailed(e) | NamingError::FaultyStreamWrite(e) =>
                        Self::Other(e),
                }
            }
        }

        // TODO: get the impl done once the implementation details of
        // `fill_arcs()` are done

        impl From<FillArcsError> for BoardError {
            fn from(value: FillArcsError) -> Self { todo!() }
        }

        pub(crate) trait Board:
            GraphBackend<Vertex: IdExt<Id = <Self as Board>::VertexId> + FieldsExt<usize, 3>>
            + for<'a> VertexIterExt<'a, Self>
            + IdExt<Id = <Self as Board>::GraphId>
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
                directed: bool,
            ) -> Result<Self, BoardError> {
                let component_ranges = normalize_board_size(&mut n1, &mut n2, &mut n3, &mut n4)?;
                let mut graph: Self = build_graph(&component_ranges)?;
                name_graph(&mut graph, &[n1, n2, n3, n4, piece.get(), wrap], directed)?;
                fill_arcs(&mut graph, piece.get(), wrap, directed)?;

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
