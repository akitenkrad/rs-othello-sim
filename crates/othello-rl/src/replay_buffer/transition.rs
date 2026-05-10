//! [`Transition`] and [`TransitionBatch`] type definitions.
//!
//! `Transition` holds a single step of experience for self-play / RL.
//! `TransitionBatch` is the batched representation returned at sampling
//! time.

use ndarray::{Array1, Array2, Array3, Array4};
use othello_core::Color;
use serde::{Deserialize, Serialize};

/// A single step of experience (transition).
///
/// - `observation`: Planes-format observation tensor `(3, H, W)` containing
///   `own / opp / legal_mask`.
/// - `action`: integer in `0..H*W+1` (the last index is `Pass`).
/// - `policy`: optional AlphaZero-style visit-count distribution of length
///   `H*W+1`.
/// - `value`: terminal reward from the player's viewpoint (`-1 / 0 / +1`)
///   or a TD target.
/// - `legal_mask`: legal-move mask at the time of observation, length
///   `H*W+1`.
/// - `side`: side to move at this transition (the observation's viewpoint).
/// - `move_number`: cumulative move number within the game.
/// - `game_id`: identifier of the originating game (for debugging).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// `(3, H, W)` Planes observation.
    pub observation: Array3<f32>,
    /// Action index.
    pub action: u32,
    /// Optional action-distribution target.
    pub policy: Option<Array1<f32>>,
    /// Value target (from the player's viewpoint).
    pub value: f32,
    /// Legal-move mask at the time of observation.
    pub legal_mask: Array1<bool>,
    /// Player whose viewpoint the observation uses.
    pub side: Color,
    /// Move number within the game.
    pub move_number: u32,
    /// Identifier of the originating game.
    pub game_id: String,
}

/// `(B, ...)` batch-output structure.
///
/// Designed to be stacked into NumPy arrays on the Python side.
#[derive(Debug, Clone)]
pub struct TransitionBatch {
    /// `(B, 3, H, W)` observation tensor.
    pub observations: Array4<f32>,
    /// `(B,)` action indices.
    pub actions: Array1<u32>,
    /// Optional `(B, H*W+1)` action distributions. `Some` only when every
    /// transition in the batch carries a policy.
    pub policies: Option<Array2<f32>>,
    /// `(B,)` value targets.
    pub values: Array1<f32>,
    /// `(B, H*W+1)` legal-move masks.
    pub legal_masks: Array2<bool>,
    /// Buffer index of each transition; used for PER priority updates.
    pub indices: Vec<usize>,
    /// `(B,)` importance-sampling weights (PER only).
    pub weights: Option<Array1<f32>>,
}

impl TransitionBatch {
    /// Returns the batch size.
    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Returns whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
