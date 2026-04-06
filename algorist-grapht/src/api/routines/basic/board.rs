use std::{
  cmp::Ordering,
  error::Error,
  fmt::{self, Debug, Display, Formatter, Write as _},
  hint,
  iter,
  mem::MaybeUninit,
  num::NonZeroIsize,
  ops::{ControlFlow, Not},
};

use itertools::Itertools;
use num_traits::AsPrimitive;
use thiserror::Error;

use crate::{
  api::{ArcAddExt, FieldsExt, GraphBackend, IdExt, VertexIterExt},
  fields_of,
};

// TODO: tweak all `unsafe` non-bounds-checking callsites with bounds-checked
// versions that run only in debug builds.

#[derive(Debug, Error)]
pub(crate) enum NormalizationError {
  #[error("allocation of output component ranges failed")]
  ComponentRangesAllocFailed,
}

pub(crate) fn normalize_board_size(
  n1: isize,
  n2: isize,
  n3: isize,
  n4: isize,
) -> Result<Vec<usize>, NormalizationError> {
  [n1, n2, n3, n4]
    .iter()
    .enumerate()
    .try_fold(
      Vec::try_with_capacity(4)
        .map_err(|_| NormalizationError::ComponentRangesAllocFailed)?,
      |mut components, (component_num, &component)| match (
        component.cmp(&0),
        component_num,
      ) {
        | (Ordering::Less | Ordering::Equal, 0) => ControlFlow::Break(Ok(
          (0..2).fold(components, |mut output, _| (output.push(8), output).1),
        )),
        | (Ordering::Less, _) => ControlFlow::Break(
          (
            components.clear(),
            components.try_reserve_exact(component.unsigned_abs()).map(|()| {
              (
                _ = [n1, n2, n3]
                  .into_iter()
                  .take(component_num)
                  .cycle()
                  .take(component.unsigned_abs())
                  .map(isize::cast_unsigned)
                  .collect_into(&mut components),
                components,
              )
                .1
            }),
          )
            .1,
        ),
        | (Ordering::Equal, _) => ControlFlow::Break(Ok(components)),
        | (Ordering::Greater, _) => ControlFlow::Continue(
          (components.push(component.cast_unsigned()), components).1,
        ),
      },
    )
    .map_continue(Ok)
    .into_value()
    .map_err(|_| NormalizationError::ComponentRangesAllocFailed)
}

#[derive(Debug, Error)]
pub(crate) enum BuildGraphError {
  #[error(
    "input component sizes produce a larger-than-signed machine word vertex \
     count"
  )]
  ComponentSizesOutOfBounds,
  #[error("failed to create graph: {0}")]
  GraphCreationFailed(#[source] Box<dyn Error>),
  #[error("auxiliary allocation failed")]
  AuxiliaryAlloc,
  #[error("writing onto the name stream for vertex ids failed")]
  StreamWrite,
  #[error(transparent)]
  FieldAccess(Box<dyn Error>),
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
  for<'a> <G as GraphBackend>::Error: 'a,
  for<'a> <<G as GraphBackend>::Vertex as FieldsExt<usize, 3>>::Error: 'a,
{
  #![expect(clippy::unit_arg, reason = "Beauty comes at a cost.")]

  iter::once((
    (0..component_range.len()).fold(
      Vec::try_with_capacity(component_range.len())
        .map_err(|_| BuildGraphError::AuxiliaryAlloc)?,
      |mut output, _| (output.push(0), output).1,
    ),
    G::new(
      component_range
        .iter()
        .try_fold(1_usize, |sum, &component| sum.checked_mul(component))
        .ok_or(BuildGraphError::ComponentSizesOutOfBounds)?,
    )
    .map_err(|e| {
      BuildGraphError::GraphCreationFailed(match Box::try_new(e) {
        | Ok(e) => e as Box<dyn Error>,
        | Err(_) => return BuildGraphError::AuxiliaryAlloc,
      })
    })?,
  ))
  .try_fold(MaybeUninit::uninit(), |mut out, (mut name_state, mut graph)| {
    Ok(
      (
        _ = graph.iter_mut().try_fold(
          // NOTE: this accounts for both the number of digits of the last
          // vertex's coordinate, as well as separation dots, like so: `1.2.3`.
          String::try_with_capacity(
            component_range
              .iter()
              .map(|&component_range| {
                (
                  // `c > 0` should hold for any component `c` in
                  // `component_range` after running `normalize_board_size()`.
                  debug_assert_ne!(component_range, 0),
                  // +1 because `ilog10()` rounds down.
                  component_range.ilog10() as usize + 1,
                )
                  .1
              })
              .sum::<usize>()
              + component_range.len(),
          )
          .map_err(|_| BuildGraphError::AuxiliaryAlloc)?,
          |mut name, vertex| {
            Ok(
              (
                name = name_state.iter().enumerate().try_fold(
                  name,
                  |mut name, (idx, component)| {
                    // TODO: the original GraphBase mentions that the first
                    // three components are saved in integer utility fields, but
                    // in theory, indices `0..3` point at the "last" three
                    // components, because `name_state` iterates from left to
                    // right in the following sequence: (a, b, ..., beta,
                    // alpha). The true "first" three components would be the
                    // last three in this iterator. For now, we're only
                    // reproducing the same behavior as the one in the original
                    // GraphBase.
                    Ok::<_, BuildGraphError>(
                      (
                        match idx {
                          | n if n != name_state.len() - 1 =>
                            write!(&mut name, "{component}."),
                          | _ => write!(&mut name, "{component}"),
                        }
                        .map_err(|_| BuildGraphError::StreamWrite)?,
                        (..3)
                          .contains(&idx)
                          .then_some(())
                          .iter()
                          .try_for_each(|()| {
                            // NOTE: we could be terser and have the inner
                            // branches' `Ok` wrapper be around the entire
                            // `match`, but that breaks rustfmt inside the error
                            // mapping call for the return value of `fields_of`.
                            // SAFETY: `idx` only ever takes on values in the
                            // range `0..3`.
                            match (
                              idx,
                              fields_of!(usize; 3 => v in G: vertex).map_err(
                                |e| {
                                  BuildGraphError::FieldAccess(
                                    match Box::try_new(e) {
                                      | Ok(e) => e as Box<dyn Error>,
                                      | Err(_) =>
                                        return BuildGraphError::AuxiliaryAlloc,
                                    },
                                  )
                                },
                              )?,
                            ) {
                              | (0, [x, ..]) => Ok(*x = *component),
                              | (1, [_, y, _]) => Ok(*y = *component),
                              | (2, [.., z]) => Ok(*z = *component),
                              | _ => unsafe { hint::unreachable_unchecked() },
                            }
                          })?,
                        name,
                      )
                        .2,
                    )
                  },
                )?,
                vertex.set_id(name.as_str()),
                name.clear(),
                name_state
                  .iter_mut()
                  .zip(component_range)
                  .rev()
                  .try_for_each(|(component, ref_component)| match (*component
                    + 1
                    == *ref_component)
                    .then(|| *component += 1)
                    .or_else(|| (*component += 1, None).1)
                  {
                    | Some(()) => ControlFlow::Continue(()),
                    | _ => ControlFlow::Break(()),
                  })
                  .into_value(),
                name,
              )
                .4,
            )
          },
        )?,
        (_ = out.write(graph), out).1,
      )
        .1,
    )
  })
  .map(|graph| unsafe { graph.assume_init() })
}

#[derive(Debug, Error)]
pub(crate) enum NamingError {
  #[error("auxiliary allocation failed")]
  AuxiliaryAlloc,
  #[error("failed to write to naming string")]
  StreamWrite,
}

pub(crate) fn name_graph<G: GraphBackend + IdExt>(
  graph: &mut G,
  params: &[isize],
  directed: bool,
) -> Result<(), NamingError>
where
  for<'a> &'a str: Into<<G as IdExt>::Id>,
{
  #![expect(clippy::unit_arg, reason = "Beauty comes at a cost.")]

  macro_rules! write_err {
    ($graph_id:expr, $($args:expr),+) => {{
      write!($graph_id, "{}", $($args),+).map_err(|_| NamingError::StreamWrite)
    }};
  }

  #[derive(Debug)]
  enum IterElem {
    Str(&'static str),
    Num(isize),
  }

  impl Display for IterElem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
      match self {
        | Self::Str(inner) => <str as Display>::fmt(inner, f),
        | Self::Num(inner) => <isize as Display>::fmt(inner, f),
      }
    }
  }

  impl From<&'static str> for IterElem {
    fn from(value: &'static str) -> Self { Self::Str(value) }
  }

  impl From<isize> for IterElem {
    fn from(value: isize) -> Self { Self::Num(value) }
  }

  impl<'a> From<&'a isize> for IterElem {
    fn from(value: &'a isize) -> Self { Self::Num(*value) }
  }

  // NOTE: The string must account for
  // (1) `board()`, and
  // (2) however as many digits each of the parameters has (considering
  //     `ilog10()` rounds downward,) and
  // (3) for both
  //     (a) the commas after each of the parameters (other than the `directed`
  //         parameter,) and
  //     (b) the extra `directed` boolean parameter (encoded as `0`/`1`.)
  iter::once("board(")
    .map_into::<IterElem>()
    .chain(params.iter().map_into())
    .chain(iter::once(isize::from(directed)).map_into())
    .chain(iter::once(")").map_into::<IterElem>())
    .try_fold(
      String::try_with_capacity(
        "board()".len()
          + params
            .iter()
            .map(|param| {
              match (
                param.checked_ilog10(),
                param.unsigned_abs().ilog10() as usize,
              ) {
                | (Some(digits), _) => digits as usize + 1,
                | (_, digits) if param.is_negative() => digits + 2,
                | (_, digits) => digits + 1,
              }
            })
            .sum::<usize>()
          + params.len()
          + 1,
      )
      .map_err(|_| NamingError::AuxiliaryAlloc)?,
      |mut graph_id, string| write_err!(graph_id, string).map(|()| graph_id),
    )
    .map(|graph_id| graph.set_id(graph_id.as_str()))
}

#[derive(Debug, Error)]
pub(crate) enum InitStateError {
  #[error(
    "failed to allocate auxiliary memory for {} elements for {}",
    .1,
    match .0 {
      InitStateErrorSrc::Wrapping => "coordinate wrapping purposes",
      InitStateErrorSrc::Motions => "coordinate state-saving purposes",
      InitStateErrorSrc::Change => "coordinate change purposes",
    }
  )]
  AuxiliaryAlloc(InitStateErrorSrc, usize),
}

#[derive(Debug)]
pub(crate) enum InitStateErrorSrc {
  Wrapping,
  Motions,
  Change,
}

pub(crate) type InitState = (Vec<bool>, Vec<isize>, Vec<usize>);

pub(crate) fn init_state(
  wrap: isize,
  dimensions: usize,
) -> Result<InitState, InitStateError> {
  // TODO: handle the case where `wrap` is negative by going straight for a full
  // wrapping vector; Possibly implemented in terms of an enumeration where it's
  // either a vector of booleans or a single boolean.

  macro_rules! gen_vec {
    ($target_len:expr => $var:tt) => {{
      Vec::try_with_capacity($target_len)
        .map(|mut out| (out.resize($target_len, 0), out).1)
        .map_err(|_| {
          InitStateError::AuxiliaryAlloc(InitStateErrorSrc::$var, $target_len)
        })?
    }};
  }

  // SAFETY: just read through the code.
  Ok((
    (0..dimensions)
      .fold(
        (
          Vec::try_with_capacity(dimensions).map_err(|_| {
            InitStateError::AuxiliaryAlloc(
              InitStateErrorSrc::Wrapping,
              dimensions,
            )
          })?,
          wrap.cast_unsigned(),
        ),
        |(should_wrap, wrap_mask), _| {
          (
            unsafe {
              iter::once(should_wrap)
                .map(|mut should_wrap| {
                  (should_wrap.push((wrap_mask & 1) != 0), should_wrap).1
                })
                .next()
                .unwrap_unchecked()
            },
            wrap_mask >> 1,
          )
        },
      )
      .0,
    gen_vec!(dimensions => Motions),
    gen_vec!(dimensions + 1 => Change),
  ))
}

#[derive(Debug, Error)]
pub(crate) enum GenMovesError {
  #[error(transparent)]
  ArcAddition(Box<dyn Error>),
  #[error("auxiliary allocation failed")]
  AuxiliaryAlloc,
}

pub(crate) fn gen_moves<G: GraphBackend + ArcAddExt>(
  graph: &mut G,
  src: usize,
  (current_state, prior_state, change): (&mut [isize], &[isize], &[isize]),
  (component_range, wrap, directed, piece): (&[usize], &[bool], bool, isize),
) -> Result<(), GenMovesError>
where
  for<'a> <G as ArcAddExt>::Error: 'a,
{
  #![expect(clippy::unit_arg, reason = "Beauty comes at a cost.")]

  // NOTE: the below cast roundtrips will not cause overflow because of the same
  // reasons as outlined in `fill_arcs()`.
  (0..usize::MAX)
    .try_for_each(|weight| {
      current_state
        .iter_mut()
        .zip(component_range.iter().map(|&range| range.cast_signed()).zip(wrap))
        .try_for_each(|(component, (max_component, &should_wrap))| {
          // SAFETY: just read through the code.
          macro_rules! normalize {
            ($iter:expr) => {
              unsafe {
                iter::once($iter)
                  .map(|()| ControlFlow::Continue(()))
                  .next()
                  .unwrap_unchecked()
              }
            };
          }

          match (
            (component.is_negative() && should_wrap),
            (*component >= max_component && should_wrap),
          ) {
            | (true, _) => normalize!(
              (*component..0)
                .step_by(max_component.cast_unsigned())
                .for_each(|_| *component += max_component)
            ),
            | (_, true) => normalize!(
              (max_component..*component)
                .rev()
                .step_by(max_component.cast_unsigned())
                .for_each(|_| *component -= max_component)
            ),
            | _ => ControlFlow::Break(Ok(())),
          }
        })?;
      match piece.is_negative().then(|| current_state.iter().ne(prior_state)) {
        | Some(true) | None => ControlFlow::Continue(()),
        | Some(false) => ControlFlow::Break(Ok(())),
      }?;
      // TODO: change `ArcAddExt` to allow adding weights to arcs/edges.

      macro_rules! new {
        ($selector:tt; $src:expr) => {{
          iter::once((
            current_state
              .iter()
              .zip(
                component_range.iter().map(|component| component.cast_signed()),
              )
              .skip(1)
              .fold(
                unsafe { *current_state.first().unwrap_unchecked() },
                |idx, (component, max_range)| max_range * idx + component,
              )
              .cast_unsigned(),
            |()| ControlFlow::Continue(()),
            |e| {
              ControlFlow::Break(Err(e).map_err(|e| {
                GenMovesError::ArcAddition(match Box::try_new(e) {
                  | Ok(e) => e as Box<dyn Error>,
                  | Err(_) => return GenMovesError::AuxiliaryAlloc,
                })
              }))
            },
          ))
          .map(|(dst, handler, error_handler)| {
            macro_rules! _spec {
              (arc) => {
                graph.new_arc($src, dst).map_or_else(error_handler, handler)
              };
              (edge) => {
                graph.new_edge($src, dst).map_or_else(error_handler, handler)
              };
            }

            _spec!($selector)
          });
        }};
      }

      match directed {
        | true => new!(arc; src),
        | false => todo!(),
      }?;
      // match (
      //   (
      //     directed,
      //     current_state
      //       .iter()
      //       .zip(
      //         component_range.iter().map(|component|
      // component.cast_signed()),       )
      //       .skip(1)
      //       .fold(
      //         unsafe { *current_state.first().unwrap_unchecked() },
      //         |idx, (component, max_range)| max_range * idx + component,
      //       )
      //       .cast_unsigned(),
      //   ),
      //   (
      //     |e| {
      //       ControlFlow::Break(Err(e).map_err(|e| {
      //         GenMovesError::ArcAddition(match Box::try_new(e) {
      //           | Ok(e) => e as Box<dyn Error>,
      //           | Err(_) => return GenMovesError::AuxiliaryAlloc,
      //         })
      //       }))
      //     },
      //     |()| ControlFlow::Continue(()),
      //   ),
      // ) {
      //   | ((true, dst), (def, map)) =>
      //     graph.new_arc(src, dst).map_or_else(def, map),
      //   | ((false, dst), (def, map)) =>
      //     graph.new_edge(src, dst).map_or_else(def, map),
      // }?;

      match piece.is_positive().then_some(Ok(())).ok_or_else(|| {
        current_state
          .iter_mut()
          .zip(change)
          .for_each(|(component, delta)| *component += delta);
      }) {
        | Ok(out) => ControlFlow::Break(out),
        | Err(()) => ControlFlow::Continue(()),
      }
    })
    .map_continue(Ok)
    .into_value()
}

#[derive(Debug, Error)]
pub(crate) enum FillArcsError {
  #[error(
    "failed to initialize auxiliary allocations to determine set of possible \
     board positions for {} elements for `{}`",
     .1,
     match .0 {
       InitStateErrorSrc::Wrapping => "coordinate wrapping purposes",
       InitStateErrorSrc::Motions => "coordinate state-saving purposes",
       InitStateErrorSrc::Change => "coordinate change purposes",
     }
  )]
  InitState(InitStateErrorSrc, usize),
  #[error("failed to perform auxiliary allocation")]
  AuxiliaryAlloc,
}

impl From<InitStateError> for FillArcsError {
  fn from(value: InitStateError) -> Self {
    match value {
      | InitStateError::AuxiliaryAlloc(src, allocation_size) =>
        Self::InitState(src, allocation_size),
    }
  }
}

// TODO: implement this once the `gen_moves()` routine is done.

impl From<GenMovesError> for FillArcsError {
  fn from(value: GenMovesError) -> Self { todo!() }
}

pub(crate) fn fill_arcs<
  G: GraphBackend + for<'a> VertexIterExt<'a, G> + ArcAddExt,
>(
  graph: &mut G,
  component_range: &[usize],
  piece: isize,
  wrap: isize,
  directed: bool,
) -> Result<(), FillArcsError>
where
  for<'a> <G as ArcAddExt>::Error: 'a,
{
  let ((wrap, mut del, mut sig), piece_unsigned) =
    (init_state(wrap, component_range.len())?, piece.unsigned_abs());
  while let ControlFlow::Break((i, d)) = del
    .iter_mut()
    .zip(sig.iter().copied().enumerate().take(component_range.len()))
    .rev()
    .try_for_each(|(d, (i, s))| {
      match (s + (*d + 1).saturating_pow(2).cast_unsigned() > piece_unsigned)
        .then(|| *d = 0)
        .or_else(|| (*d += 1, None).1)
      {
        | Some(()) => ControlFlow::Continue(()),
        | _ => ControlFlow::Break((i, &*d)),
      }
    })
  {
    // SAFETY: `i` is always within bounds by virtue of being a result of an
    // `enumerate()` call on the `sig` collection's iterator. `i + 1` is always
    // within bounds by virtue of `sig` always being one element larger than
    // `del`, and the above iteration being dominated by the latter (see the
    // `take()` call on the iterator produced by `sig` and the allocation sizes
    // in the `init_state()` routine.)
    // NOTE: we skip however as many elements there are until reaching index `i
    // + 1` (i.e. `i + 1` elements, corresponding to the `i + 1` indices from
    // `0..=i`.)
    ((0..sig.len()).skip(i + 1).fold(
      unsafe { *sig.get_unchecked(i) },
      |target_sig, s| unsafe {
        (
          *sig.get_unchecked_mut(s) =
            target_sig + d.saturating_pow(2).cast_unsigned(),
          target_sig,
        )
          .1
      },
    ) < piece_unsigned)
      .not()
      .then_some(())
      .iter()
      .cycle()
      .try_for_each(|()| {
        macro_rules! gen_vec {
          () => {{
            match Vec::try_with_capacity(component_range.len())
              .map(|mut out| {
                (out.resize(component_range.len(), 0_isize), out).1
              })
              .map_err(|_| FillArcsError::AuxiliaryAlloc)
            {
              | Ok(out) => out,
              // NOTE: the below line uses a new `Result<_, T(err)>` instead of
              // repurposing the existing `Err` because otherwise we would carry
              // through the same generic type for the `Ok` variant, when the
              // overarching `Ok` variant that the closure's returned
              // `ControlFlow` wraps actually requires a different type
              // for the `Ok` variant that is inferred later on.
              | Err(err) => return ControlFlow::Break(Err(err)),
            }
          }};
        }

        (0..graph.iter().count()).try_fold(
          (gen_vec!(), gen_vec!()),
          |(mut current_state, mut post_state), vertex_idx| {
            post_state
              .iter_mut()
              .zip(
                current_state
                  .iter()
                  .zip(&del)
                  .map(|(component, change)| component + change),
              )
              .for_each(|(post_move, pre_move)| *post_move = pre_move);
            match gen_moves(
              graph,
              vertex_idx,
              (&mut post_state, &current_state, &del),
              (component_range, &wrap, directed, piece),
            ) {
              | Ok(()) => ControlFlow::Continue(()),
              // NOTE: see the note left on the `gen_vector` macro definition.
              | Err(err) => ControlFlow::Break(Err(err.into())),
            }?;
            // NOTE: casting here won't wrap because all elements from
            // `component_range` are sourced from `normalize_board_size()`,
            // which itself takes in `isize` values and can only ever produce
            // (positive) coordinate ranges that can be denoted by an `isize`.
            // `unsigned_abs()` on an `isize` would always yield values within
            // a `usize`'s range; Ergo, this is sound.
            current_state
              .iter_mut()
              .zip(component_range.iter().map(|range| range.cast_signed()))
              .rev()
              .try_for_each(|(x, max_x)| {
                match (*x + 1).cmp(&max_x) {
                  | Ordering::Equal => (*x = 0, ControlFlow::Continue(())),
                  | _ => (*x += 1, ControlFlow::Break(())),
                }
                .1
              })
              .into_value();

            ControlFlow::Continue((current_state, post_state))
          },
        )?;

        match del
          .iter_mut()
          .enumerate()
          .rev()
          .try_fold(usize::default(), |_, (i, d)| {
            match (*d <= 0).then(|| *d = d.wrapping_neg()).or_else(|| {
              (unsafe { *sig.get_unchecked(i) } != 0)
                .then(|| *d = d.wrapping_neg())
                .filter(|()| false)
            }) {
              | Some(()) =>
                ControlFlow::Continue(unsafe { *sig.get_unchecked(i) }),
              | _ => ControlFlow::Break(unsafe { *sig.get_unchecked(i) }),
            }
          })
          .into_value()
        {
          | 0 => ControlFlow::Break(Ok(())),
          | _ => ControlFlow::Continue(()),
        }
      })
      .map_continue(Ok)
      .into_value()?;
  }

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
  #[error(
    "failed to normalize component range sizes for d-dimensional adjacency \
     matrix"
  )]
  Normalization,
  #[error("failed to build graph: {0}")]
  GraphBuild(GraphBuildErrorKind),
  #[error("failed to assign names to graph vertices")]
  GraphNaming,
  /// If you hit this error, some implementation detail hidden from the reach of
  /// library users has failed.
  ///
  /// The error is fairly opaque and thus not meant to communicate anything but
  /// the fact that an unrecoverable error took place, and its meaning is not
  /// exposed in the public API.
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
      | e => (Box::new(e) as Box<dyn Error>).into(),
    }
  }
}

impl From<NamingError> for BoardError {
  fn from(_: NamingError) -> Self { Self::GraphNaming }
}

// TODO: get the impl done once the implementation details of `fill_arcs()` are
// done

impl From<FillArcsError> for BoardError {
  fn from(value: FillArcsError) -> Self { todo!() }
}

// TODO: finish the below API to allow for more a mask larger than 32/64-bits to
// be used for configuring which coordinate component gets wrapped; currently
// we're using `isize` for `wrap` in `Board::board()`.

#[derive(Debug)]
pub(crate) enum WrapBuilderRepr {
  SpecificComponents(Vec<usize>),
  AllComponents,
}

#[derive(Debug, Error)]
pub(crate) enum WrapBuilderError {
  #[error("auxiliary allocation failed")]
  AuxiliaryAlloc,
}

#[derive(Debug, Default)]
pub(crate) struct WrapBuilder(pub(crate) Option<WrapBuilderRepr>);

impl WrapBuilder {
  fn new<T: Into<Option<impl AsPrimitive<usize>>>>(
    dimensions: T,
  ) -> Result<Self, WrapBuilderError> {
    Ok(Self(if let Some(dims) = dimensions.into() {
      Some(WrapBuilderRepr::SpecificComponents(
        Vec::try_with_capacity(dims.as_())
          .map_err(|_| WrapBuilderError::AuxiliaryAlloc)?,
      ))
    } else {
      Some(WrapBuilderRepr::AllComponents)
    }))
  }

  fn add_wrapping(
    &mut self,
    component: impl AsPrimitive<usize>,
  ) -> Result<&mut Self, WrapBuilderError> {
    if let Some(inner) = &mut self.0 {
      todo!()
    } else {
      todo!()
    }

    Ok(self)
  }
}

pub(crate) trait Board:
  GraphBackend<
    Vertex: IdExt<Id = <Self as Board>::VertexId> + FieldsExt<usize, 3>,
  > + for<'a> VertexIterExt<'a, Self>
  + IdExt<Id = <Self as Board>::GraphId>
  + ArcAddExt
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
    n1: isize,
    n2: isize,
    n3: isize,
    n4: isize,
    piece: NonZeroIsize,
    wrap: isize,
    directed: bool,
  ) -> Result<Self, BoardError> {
    let component_ranges = normalize_board_size(n1, n2, n3, n4)?;
    let mut graph: Self = build_graph(&component_ranges)?;
    name_graph(&mut graph, &[n1, n2, n3, n4, piece.get(), wrap], directed)?;
    fill_arcs(&mut graph, &component_ranges, piece.get(), wrap, directed)?;

    Ok(graph)
  }

  fn complete() {}

  fn transitive() {}

  fn empty() {}

  fn circuit() {}

  fn cycle() {}
}
