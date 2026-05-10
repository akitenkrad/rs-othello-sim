//! [`PrioritizedReplayBuffer`]: Prioritized Experience Replay (Schaul et al., 2015).
//!
//! 各 transition に priority $p_i$ を持たせ，
//! $P(i) = p_i^\alpha / \sum_j p_j^\alpha$ で比例サンプリングする．
//! TD error $\delta$ を観測したら $p_i = (|\delta| + \epsilon)$ で更新する．
//! IS weight $w_i = (N \cdot P(i))^{-\beta}$ をバッチ最大値で正規化して返す．
//!
//! 効率的な比例サンプリングは [`crate::replay_buffer::sum_tree::SumTree`] を用いる．

use rand::Rng;

use crate::replay_buffer::sum_tree::SumTree;
use crate::replay_buffer::transition::{Transition, TransitionBatch};
use crate::replay_buffer::{ReplayBuffer, ReplayError, stack_transitions};

/// Prioritized Experience Replay buffer．
///
/// `alpha = 0` で uniform，`alpha = 1` で priority に比例．
/// `beta` は学習中に 1.0 まで上昇させる ( IS bias 補正)．
pub struct PrioritizedReplayBuffer {
    capacity: usize,
    buffer: Vec<Option<Transition>>,
    tree: SumTree,
    alpha: f32,
    beta: f32,
    beta_increment: f32,
    epsilon: f32,
    max_priority: f32,
    cursor: usize,
    size: usize,
}

impl std::fmt::Debug for PrioritizedReplayBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrioritizedReplayBuffer")
            .field("capacity", &self.capacity)
            .field("size", &self.size)
            .field("alpha", &self.alpha)
            .field("beta", &self.beta)
            .field("beta_increment", &self.beta_increment)
            .field("epsilon", &self.epsilon)
            .field("max_priority", &self.max_priority)
            .field("tree_total", &self.tree.total())
            .finish()
    }
}

impl PrioritizedReplayBuffer {
    /// 容量 `capacity` の PER buffer を作る．既定: $\alpha=0.6, \beta_0=0.4, \beta_{inc}=10^{-3}, \epsilon=10^{-6}$．
    ///
    /// # Panics
    ///
    /// `capacity == 0` で panic．
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ReplayBuffer capacity must be > 0");
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }
        Self {
            capacity,
            buffer,
            tree: SumTree::new(capacity),
            alpha: 0.6,
            beta: 0.4,
            beta_increment: 0.001,
            epsilon: 1e-6,
            max_priority: 1.0,
            cursor: 0,
            size: 0,
        }
    }

    /// `alpha` を設定する ( builder)．
    #[must_use]
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// 初期 `beta` を設定する ( builder)．
    #[must_use]
    pub fn with_beta(mut self, beta: f32) -> Self {
        self.beta = beta;
        self
    }

    /// `beta` の増分を設定する ( builder)．
    #[must_use]
    pub fn with_beta_increment(mut self, increment: f32) -> Self {
        self.beta_increment = increment;
        self
    }

    /// `epsilon` (priority floor) を設定する ( builder)．
    #[must_use]
    pub fn with_epsilon(mut self, eps: f32) -> Self {
        self.epsilon = eps;
        self
    }

    /// 現在の `beta`．
    #[inline]
    #[must_use]
    pub fn beta(&self) -> f32 {
        self.beta
    }

    /// 現在の `alpha`．
    #[inline]
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// 現在記録されている最大 priority．
    #[inline]
    #[must_use]
    pub fn max_priority(&self) -> f32 {
        self.max_priority
    }
}

impl ReplayBuffer for PrioritizedReplayBuffer {
    fn push(&mut self, transition: Transition) {
        let leaf_priority = self.max_priority.powf(self.alpha);
        // Slot は SumTree のリングと同期させる．
        let leaf_id = self.cursor;
        self.buffer[leaf_id] = Some(transition);
        self.tree.update(leaf_id, leaf_priority);
        self.cursor = (self.cursor + 1) % self.capacity;
        if self.size < self.capacity {
            self.size += 1;
        }
    }

    fn len(&self) -> usize {
        self.size
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn sample(
        &mut self,
        batch_size: usize,
        rng: &mut dyn rand::RngCore,
    ) -> Result<TransitionBatch, ReplayError> {
        let n = self.size;
        if n == 0 {
            return Err(ReplayError::Empty);
        }
        if batch_size == 0 || batch_size > n {
            return Err(ReplayError::BatchTooLarge {
                requested: batch_size,
                available: n,
            });
        }
        let total = self.tree.total();
        if !total.is_finite() || total <= 0.0 {
            // すべての priority が 0 / NaN．無効状態ではあるが，
            // 実用上は発生しにくい．Empty として扱う．
            return Err(ReplayError::Empty);
        }
        // 層化サンプリング ( segment) で比例サンプリング．論文式 ( 6) の "stratified sampling"．
        let segment = total / batch_size as f32;
        let mut indices: Vec<usize> = Vec::with_capacity(batch_size);
        let mut leaf_priorities: Vec<f32> = Vec::with_capacity(batch_size);
        let mut refs: Vec<&Transition> = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let lo = segment * i as f32;
            let hi = segment * (i + 1) as f32;
            // hi が total を超えないようにクランプ
            let hi = hi.min(total);
            let lo = lo.min(hi);
            let v: f32 = if hi > lo { rng.gen_range(lo..hi) } else { lo };
            let (leaf_id, p) = self.tree.get(v);
            // 安全対策: leaf に transition がなければ別の値で再試行．
            // ( push されていない leaf は priority が 0 なので通常選ばれない．)
            let chosen = if self.buffer[leaf_id].is_some() {
                (leaf_id, p)
            } else {
                let v_retry = rng.gen_range(0.0..total);
                self.tree.get(v_retry)
            };
            indices.push(chosen.0);
            leaf_priorities.push(chosen.1);
            refs.push(
                self.buffer[chosen.0]
                    .as_ref()
                    .ok_or(ReplayError::IndexOutOfBounds(chosen.0, n))?,
            );
        }

        // IS weight: w_i = (N * P(i))^(-beta)．バッチ最大値で正規化する．
        let mut weights = Vec::with_capacity(batch_size);
        let n_f = n as f32;
        for &p in &leaf_priorities {
            let prob = if total > 0.0 { p / total } else { 0.0 };
            let w = if prob > 0.0 {
                (n_f * prob).powf(-self.beta)
            } else {
                0.0
            };
            weights.push(w);
        }
        let max_w = weights
            .iter()
            .copied()
            .fold(0.0f32, |a, b| if b > a { b } else { a });
        if max_w > 0.0 {
            for w in &mut weights {
                *w /= max_w;
            }
        }

        let weights_arr = ndarray::Array1::from(weights);

        let (observations, actions, policies, values, legal_masks) = stack_transitions(&refs)?;

        // beta は次回サンプル時に向けて少しずつ増加させる．
        self.beta = (self.beta + self.beta_increment).min(1.0);

        Ok(TransitionBatch {
            observations,
            actions,
            policies,
            values,
            legal_masks,
            indices,
            weights: Some(weights_arr),
        })
    }

    fn update_priorities(
        &mut self,
        indices: &[usize],
        priorities: &[f32],
    ) -> Result<(), ReplayError> {
        if indices.len() != priorities.len() {
            return Err(ReplayError::PriorityLengthMismatch {
                indices: indices.len(),
                priorities: priorities.len(),
            });
        }
        for (&idx, &p) in indices.iter().zip(priorities.iter()) {
            if p < 0.0 || !p.is_finite() {
                return Err(ReplayError::NegativePriority(p));
            }
            if idx >= self.capacity {
                return Err(ReplayError::IndexOutOfBounds(idx, self.capacity));
            }
            // |delta| + epsilon を内部で持ち，alpha を掛けて leaf に書き込む．
            let priority = p + self.epsilon;
            if priority > self.max_priority {
                self.max_priority = priority;
            }
            let leaf_priority = priority.powf(self.alpha);
            self.tree.update(idx, leaf_priority);
        }
        Ok(())
    }

    fn clear(&mut self) {
        for slot in &mut self.buffer {
            *slot = None;
        }
        self.tree.clear();
        self.cursor = 0;
        self.size = 0;
        self.max_priority = 1.0;
    }
}
