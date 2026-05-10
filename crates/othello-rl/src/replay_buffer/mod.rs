//! Replay buffer for self-play / RL training pipelines.
//!
//! `othello-rl::replay_buffer` provides a [`ReplayBuffer`] trait shared between a
//! ring-buffer [`UniformReplayBuffer`] ( for vanilla Q-learning style training)
//! and a [`PrioritizedReplayBuffer`] ( PER, Schaul et al., 2015) backed by a
//! [`SumTree`] for efficient proportional sampling．
//!
//! Use [`transitions_from_record`] / [`transitions_from_record_both_sides`] to
//! convert an [`othello_io::GameRecord`] into per-step [`Transition`]s with
//! proper terminal values from the agent's viewpoint．

pub mod prioritized;
pub mod sum_tree;
pub mod transition;
pub mod uniform;

mod from_record;

pub use from_record::{transitions_from_record, transitions_from_record_both_sides};
pub use prioritized::PrioritizedReplayBuffer;
pub use sum_tree::SumTree;
pub use transition::{Transition, TransitionBatch};
pub use uniform::UniformReplayBuffer;

use ndarray::{Array1, Array2, Array4, Axis};
use thiserror::Error;

/// Common error type for replay buffers.
#[derive(Debug, Error)]
pub enum ReplayError {
    /// The buffer is empty.
    #[error("buffer is empty")]
    Empty,
    /// The requested batch size exceeds the current buffer length (or is
    /// zero).
    #[error("requested batch size {requested} exceeds buffer length {available}")]
    BatchTooLarge {
        /// Requested batch size.
        requested: usize,
        /// Number of transitions currently in the buffer.
        available: usize,
    },
    /// `update_priorities` was called with mismatched `indices` and
    /// `priorities` lengths.
    #[error("priority arrays must have equal length: indices={indices}, priorities={priorities}")]
    PriorityLengthMismatch {
        /// Length of the `indices` array.
        indices: usize,
        /// Length of the `priorities` array.
        priorities: usize,
    },
    /// Priority is negative or non-finite.
    #[error("priority must be non-negative and finite, got {0}")]
    NegativePriority(f32),
    /// An index passed to `update_priorities` is out of range.
    #[error("index {0} out of bounds (buffer length {1})")]
    IndexOutOfBounds(usize, usize),
    /// Transition observations or masks within a batch have inconsistent
    /// shapes.
    #[error("inconsistent observation shape across transitions in the batch")]
    ShapeMismatch,
    /// The game record is corrupt or violates the rules.
    #[error("invalid record: {0}")]
    InvalidRecord(String),
}

/// Common interface for replay buffers.
///
/// `Send` is required because flushes are expected from self-play workers.
pub trait ReplayBuffer: Send {
    /// Adds a single transition.
    fn push(&mut self, transition: Transition);

    /// Returns the number of transitions currently in the buffer.
    fn len(&self) -> usize;

    /// Returns the buffer capacity.
    fn capacity(&self) -> usize;

    /// Returns whether the buffer is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the buffer is filled to capacity.
    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Samples `batch_size` transitions. The `rng` is injected externally
    /// to keep sampling reproducible.
    fn sample(
        &mut self,
        batch_size: usize,
        rng: &mut dyn rand::RngCore,
    ) -> Result<TransitionBatch, ReplayError>;

    /// For PER, updates priorities with values such as TD-error magnitudes.
    /// A no-op for uniform replay buffers.
    fn update_priorities(
        &mut self,
        indices: &[usize],
        priorities: &[f32],
    ) -> Result<(), ReplayError>;

    /// Clears the buffer.
    fn clear(&mut self);
}

/// Return-value tuple of `stack_transitions`.
type StackedBatch = (
    Array4<f32>,
    Array1<u32>,
    Option<Array2<f32>>,
    Array1<f32>,
    Array2<bool>,
);

/// Stacks a slice of `Transition` references into
/// `(observations, actions, policies, values, legal_masks)`.
///
/// If any transition in the batch has `policy = None`, the returned
/// `policies` is `None`. Shape mismatches yield
/// [`ReplayError::ShapeMismatch`].
pub(crate) fn stack_transitions(refs: &[&Transition]) -> Result<StackedBatch, ReplayError> {
    if refs.is_empty() {
        return Err(ReplayError::Empty);
    }
    let first = refs[0];
    let obs_shape = first.observation.shape().to_vec();
    let mask_len = first.legal_mask.len();
    let policy_len = first.policy.as_ref().map(|p| p.len());
    let mut all_have_policy = first.policy.is_some();
    for &t in &refs[1..] {
        if t.observation.shape() != obs_shape.as_slice() {
            return Err(ReplayError::ShapeMismatch);
        }
        if t.legal_mask.len() != mask_len {
            return Err(ReplayError::ShapeMismatch);
        }
        match (&policy_len, &t.policy) {
            (Some(l), Some(p)) if p.len() == *l => {}
            (Some(_), None) => {
                all_have_policy = false;
            }
            (None, _) => {
                all_have_policy = false;
            }
            _ => return Err(ReplayError::ShapeMismatch),
        }
    }

    let batch = refs.len();
    let c = obs_shape[0];
    let h = obs_shape[1];
    let w = obs_shape[2];
    let mut observations = Array4::<f32>::zeros((batch, c, h, w));
    let mut actions = Array1::<u32>::zeros(batch);
    let mut values = Array1::<f32>::zeros(batch);
    let mut legal_masks = Array2::<bool>::default((batch, mask_len));
    let mut policies = if all_have_policy {
        Some(Array2::<f32>::zeros((batch, policy_len.unwrap_or(0))))
    } else {
        None
    };
    for (i, t) in refs.iter().enumerate() {
        observations
            .index_axis_mut(Axis(0), i)
            .assign(&t.observation);
        actions[i] = t.action;
        values[i] = t.value;
        legal_masks.index_axis_mut(Axis(0), i).assign(&t.legal_mask);
        if let (Some(out), Some(p)) = (policies.as_mut(), t.policy.as_ref()) {
            out.index_axis_mut(Axis(0), i).assign(p);
        }
    }
    Ok((observations, actions, policies, values, legal_masks))
}
