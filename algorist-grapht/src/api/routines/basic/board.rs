use std::{
  alloc::{AllocError, Allocator, Global},
  any::Any,
  cmp::Ordering,
  collections::TryReserveError,
  error::Error,
  fmt::{self, Debug, Display, Formatter, Write as _},
  hint,
  num::NonZeroIsize,
  ops::{ControlFlow, Not},
};

use num_traits::AsPrimitive;
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
  n1: isize,
  n2: isize,
  n3: isize,
  n4: isize,
) -> Result<Vec<usize>, NormalizationError> {
  Ok(
    [n1, n2, n3, n4]
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
              [n1, n2, n3]
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
      .into_value()?,
  )
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

impl From<AllocError> for BuildGraphError {
  fn from(_: AllocError) -> Self { Self::AuxiliaryAllocFailed }
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
      let ext: Box<dyn Error> = match Box::try_new(e) {
        | Ok(e) => e,
        | Err(_) => return BuildGraphError::AuxiliaryAllocFailed,
      };

      BuildGraphError::GraphCreationFailed(ext)
    })?,
  );
  graph.iter_mut().try_fold(
    // Must account for both the number of digits of the last vertex's
    // coordinate, as well as the separation dots.
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
            // TODO: the original GraphBase mentions that the first three
            // components are saved in integer utility fields, but in theory,
            // indices `0..3` point at the "last" three components, because
            // `name_state` iterates from left to right in the following
            // sequence: (a, b, ..., beta, alpha). The true "first" three
            // components would be the last three in this iterator. For now,
            // we're only reproducing the same behavior as the one in the
            // original GraphBase.
            (..3).contains(&idx).then_some(()).iter().try_for_each(|()| {
              let [x, y, z] =
                fields_of!(usize; 3 => v in G: vertex).map_err(|e| {
                  let ext: Box<dyn Error> = match Box::try_new(e) {
                    | Ok(e) => e,
                    | Err(_) => return BuildGraphError::AuxiliaryAllocFailed,
                  };

                  BuildGraphError::WrongFieldAccess(ext)
                })?;
              match idx {
                | 0 => *x = *component,
                | 1 => *y = *component,
                | 2 => *z = *component,
                // SAFETY: here `idx` only ever takes on values in the range
                // `0..3`.
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
        .zip(component_range)
        .rev()
        .try_for_each(|(component, ref_component)| {
          if *component + 1 == *ref_component {
            (*component = 0, ControlFlow::Continue(()))
          } else {
            (*component += 1, ControlFlow::Break(()))
          }
          .1
        })
        .into_value();

      Ok::<_, BuildGraphError>(name)
    },
  )?;

  Ok(graph)
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
  // The string must account for `board()`, however as many digits each of the
  // parameters has (considering `ilog10()` rounds downward,) and for both the
  // commas after each of the parameters (other than the `directed` parameter,)
  // and the extra `directed` boolean parameter.
  let mut graph_id = String::try_with_capacity(
    "board()".len()
      + params
        .iter()
        .map(|param| {
          if let Some(digits) = param.checked_ilog10() {
            digits as usize + 1
          } else {
            let digits = param.unsigned_abs().ilog10() as usize;

            if param.is_negative() { digits + 2 } else { digits + 1 }
          }
        })
        .sum::<usize>()
      + 1
      + params.len(),
  )
  .map_err(|_| NamingError::AuxiliaryAlloc)?;

  macro_rules! write_err {
    ($($args:expr),+) => {{
      write!(graph_id, $($args),+).map_err(|_| NamingError::StreamWrite)
    }};
  }

  write_err!("board(")?;
  params.iter().try_for_each(|param| write_err!("{param},"))?;
  write_err!("{})", if directed { "1" } else { "0" })?;
  graph.set_id(graph_id.as_str());

  Ok(())
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

pub(crate) type InitState = (Vec<bool>, Vec<usize>, Vec<usize>);

pub(crate) fn init_state(
  wrap: isize,
  dimensions: usize,
) -> Result<InitState, InitStateError> {
  // TODO: handle the case where `wrap` is negative by going straight for a full
  // wrapping vector; Possibly implemented in terms of an enumeration where it's
  // either a vector of booleans or a single boolean.

  macro_rules! gen_vector {
    ($target_len:expr => $var:tt) => {{
      Vec::try_with_capacity($target_len)
        .map(|mut out| {
          out.resize($target_len, 0);

          out
        })
        .map_err(|_| {
          InitStateError::AuxiliaryAlloc(InitStateErrorSrc::$var, $target_len)
        })?
    }};
  }

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
        |(mut should_wrap, wrap_mask), _| {
          (
            should_wrap.push((wrap_mask & 1) != 0),
            (should_wrap, wrap_mask >> 1),
          )
            .1
        },
      )
      .0,
    gen_vector!(dimensions => Motions),
    gen_vector!(dimensions + 1 => Change),
  ))
}

#[derive(Debug, Error)]
pub(crate) enum GenMovesError {}

pub(crate) fn gen_moves<G: GraphBackend + for<'a> VertexIterExt<'a, G>>(
  vertex: usize,
  buf: &mut [usize],
  component_state: &[usize],
  directed: bool,
) -> Result<(), GenMovesError> {
  todo!();

  Ok(())
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

impl From<GenMovesError> for FillArcsError {
  fn from(value: GenMovesError) -> Self { todo!() }
}

pub(crate) fn fill_arcs<G: GraphBackend + for<'a> VertexIterExt<'a, G>>(
  graph: &mut G,
  component_range: &[usize],
  piece: isize,
  wrap: isize,
  directed: bool,
) -> Result<(), FillArcsError> {
  let ((wr, mut del, mut sig), piece) =
    (init_state(wrap, component_range.len())?, piece.unsigned_abs());
  while let ControlFlow::Break((i, d)) = del
    .iter_mut()
    .zip(sig.iter().copied().enumerate().take(component_range.len()))
    .rev()
    .try_for_each(|(d, (i, s))| {
      if s + (*d + 1).saturating_pow(2) > piece {
        (*d = 0, ControlFlow::Continue(()))
      } else {
        (*d += 1, ControlFlow::Break((i, &*d)))
      }
      .1
    })
  {
    // SAFETY: `i` is always within bounds by virtue of being a result of an
    // `enumerate()` call on the `sig` collection's iterator. `i + 1` is always
    // within bounds by virtue of `sig` always being one element larger than
    // `del`, and the above iteration being dominated by the latter (see the
    // `take()` call on the iterator produced by `sig` and the allocation sizes
    // in the `init_state()` routine.)

    let ((), target_sig) = unsafe {
      (
        *sig.get_unchecked_mut(i + 1) =
          sig.get_unchecked(i) + d.saturating_pow(2),
        *sig.get_unchecked(i + 1),
      )
    };
    let ((), ref_elem) = unsafe {
      (*sig.get_unchecked_mut(i + 1) = target_sig, *sig.get_unchecked(i))
    };
    // +2 here to account for (1) element indices being 0-indexed and the
    // `skip()` method working on a 1-indexed basis, and (2) the fact that only
    // elements coming right at after the index we just modified above should be
    // affected (i.e. the change should only carry down elements coming *after*
    // the element at index `i + 1`.)
    sig.iter_mut().skip(i + 2).for_each(|s| *s = ref_elem);
    // NOTE: this may not be reordered. Do not attempt to reorder this because
    // last line's iteration seems like it could be skipped. Last line's
    // iteration's side effects on the `sig` collection are key, irrespective of
    // whether the carried through value thus far (i.e. the solution to `del[0]
    // + del[1] + ... + del[d - 1] = p` for all non-zero `del` elements) turns
    // out to actually yield a solution for `p`.
    (target_sig < piece).not().then_some(()).iter().try_for_each(|()| {
      (0..graph.iter().count()).try_fold(
        (
          Vec::try_with_capacity(component_range.len())
            .map(|mut out| {
              out.resize(component_range.len(), 0);

              out
            })
            .map_err(|_| FillArcsError::AuxiliaryAlloc)?,
          Vec::try_with_capacity(component_range.len())
            .map(|mut out| {
              out.resize(component_range.len(), 0);

              out
            })
            .map_err(|_| FillArcsError::AuxiliaryAlloc)?,
        ),
        |(mut current_state, mut post_state), vertex_idx| {
          // `post_state` here serves the purpose of a buffer that holds the
          // coordinates of `current_state` during move generation within
          // `gen_moves()`.
          post_state
            .iter_mut()
            .zip(
              current_state
                .iter()
                .zip(del.iter())
                .map(|(component, change)| component + change),
            )
            .for_each(|(post_move, pre_move)| *post_move = pre_move);
          gen_moves::<G>(
            vertex_idx, &mut post_state, &current_state, directed,
          )?;
          current_state
            .iter_mut()
            .zip(component_range)
            .rev()
            .try_for_each(|(x, max_x)| {
              if *x + 1 == *max_x {
                (*x = 0, ControlFlow::Continue(()))
              } else {
                (*x += 1, ControlFlow::Break(()))
              }
              .1
            })
            .into_value();

          Ok::<_, FillArcsError>((current_state, post_state))
        },
      )?;

      Ok::<_, FillArcsError>(())
    })?;
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
      | e => {
        let output: Box<dyn Error> = Box::new(e);

        output.into()
      },
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
