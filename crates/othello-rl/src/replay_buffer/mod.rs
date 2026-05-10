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

/// Replay buffer の共通エラー型．
#[derive(Debug, Error)]
pub enum ReplayError {
    /// バッファに何も入っていない．
    #[error("buffer is empty")]
    Empty,
    /// 要求されたバッチサイズが現在のバッファ件数より大きい ( または 0)．
    #[error("requested batch size {requested} exceeds buffer length {available}")]
    BatchTooLarge {
        /// 要求バッチサイズ．
        requested: usize,
        /// バッファに入っている件数．
        available: usize,
    },
    /// `update_priorities` で indices と priorities の長さが不一致．
    #[error("priority arrays must have equal length: indices={indices}, priorities={priorities}")]
    PriorityLengthMismatch {
        /// indices 配列長．
        indices: usize,
        /// priorities 配列長．
        priorities: usize,
    },
    /// priority が負または非有限．
    #[error("priority must be non-negative and finite, got {0}")]
    NegativePriority(f32),
    /// `update_priorities` で渡された index が範囲外．
    #[error("index {0} out of bounds (buffer length {1})")]
    IndexOutOfBounds(usize, usize),
    /// バッチ内 transition の観測 / マスク shape が一貫していない．
    #[error("inconsistent observation shape across transitions in the batch")]
    ShapeMismatch,
    /// 棋譜が壊れている / ルール違反．
    #[error("invalid record: {0}")]
    InvalidRecord(String),
}

/// Replay buffer の共通インターフェース．
///
/// `Send` を要求するのは self-play ワーカからの flush を想定するため．
pub trait ReplayBuffer: Send {
    /// 1 件の transition を追加する．
    fn push(&mut self, transition: Transition);

    /// 現在のバッファ内件数．
    fn len(&self) -> usize;

    /// 容量．
    fn capacity(&self) -> usize;

    /// 空かどうか．
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 容量上限まで埋まっているかどうか．
    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// `batch_size` 件サンプリングする．`rng` は外部注入で再現性を確保する．
    fn sample(
        &mut self,
        batch_size: usize,
        rng: &mut dyn rand::RngCore,
    ) -> Result<TransitionBatch, ReplayError>;

    /// PER の場合，TD error などを priority として更新する．
    /// uniform の場合は no-op．
    fn update_priorities(
        &mut self,
        indices: &[usize],
        priorities: &[f32],
    ) -> Result<(), ReplayError>;

    /// バッファを空にする．
    fn clear(&mut self);
}

/// `stack_transitions` の戻り値タプル．
type StackedBatch = (
    Array4<f32>,
    Array1<u32>,
    Option<Array2<f32>>,
    Array1<f32>,
    Array2<bool>,
);

/// `Transition` の参照スライスから (observations, actions, policies, values, legal_masks) を stack する．
///
/// `policy` がバッチ内で 1 件でも `None` の場合，戻り値の `policies` は `None`．
/// shape が一致しない場合は [`ReplayError::ShapeMismatch`]．
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
