//! [`UniformReplayBuffer`]: a ring buffer with uniform sampling.

use rand::Rng;

use crate::replay_buffer::transition::{Transition, TransitionBatch};
use crate::replay_buffer::{ReplayBuffer, ReplayError, stack_transitions};

/// Uniform-sampling replay buffer over a FIFO ring.
///
/// On capacity overflow the oldest element is overwritten (ring buffer).
#[derive(Debug)]
pub struct UniformReplayBuffer {
    capacity: usize,
    buffer: Vec<Transition>,
    cursor: usize,
    full: bool,
}

impl UniformReplayBuffer {
    /// Creates a buffer with the given `capacity`.
    ///
    /// # Panics
    ///
    /// Panics if `capacity == 0`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ReplayBuffer capacity must be > 0");
        Self {
            capacity,
            buffer: Vec::with_capacity(capacity),
            cursor: 0,
            full: false,
        }
    }
}

impl ReplayBuffer for UniformReplayBuffer {
    fn push(&mut self, transition: Transition) {
        if self.cursor < self.buffer.len() {
            self.buffer[self.cursor] = transition;
        } else {
            self.buffer.push(transition);
        }
        self.cursor += 1;
        if self.cursor >= self.capacity {
            self.cursor = 0;
            self.full = true;
        }
    }

    fn len(&self) -> usize {
        if self.full {
            self.capacity
        } else {
            self.cursor
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn sample(
        &mut self,
        batch_size: usize,
        rng: &mut dyn rand::RngCore,
    ) -> Result<TransitionBatch, ReplayError> {
        let n = self.len();
        if n == 0 {
            return Err(ReplayError::Empty);
        }
        if batch_size == 0 || batch_size > n {
            return Err(ReplayError::BatchTooLarge {
                requested: batch_size,
                available: n,
            });
        }

        let mut indices: Vec<usize> = Vec::with_capacity(batch_size);
        let mut refs: Vec<&Transition> = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let i = rng.gen_range(0..n);
            indices.push(i);
            refs.push(&self.buffer[i]);
        }
        let (observations, actions, policies, values, legal_masks) = stack_transitions(&refs)?;

        Ok(TransitionBatch {
            observations,
            actions,
            policies,
            values,
            legal_masks,
            indices,
            weights: None,
        })
    }

    fn update_priorities(&mut self, _: &[usize], _: &[f32]) -> Result<(), ReplayError> {
        Ok(())
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.full = false;
    }
}
