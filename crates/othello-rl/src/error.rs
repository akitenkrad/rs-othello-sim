//! [`RlError`]: error type for the RL environments.

use othello_core::OthelloError;
use othello_player::PlayerError;
use thiserror::Error;

/// Errors that may occur in the RL environments.
#[derive(Debug, Error)]
pub enum RlError {
    /// The action passed to `step` is illegal in the current state.
    #[error("illegal action: {action} (legal_count={legal_count})")]
    IllegalAction {
        /// Action index.
        action: u32,
        /// Number of legal moves in the state.
        legal_count: u32,
    },

    /// Action index is outside `0..space_size`.
    #[error("action {action} out of range (max {max})")]
    OutOfRange {
        /// Provided action.
        action: u32,
        /// Exclusive upper bound.
        max: u32,
    },

    /// `step` was called when it is not the agent's turn (a light
    /// safety check for the multi-agent environment).
    #[error("not your turn: side_to_move={side:?}, agent={agent:?}")]
    NotYourTurn {
        /// Current side to move.
        side: othello_core::Color,
        /// Agent color.
        agent: othello_core::Color,
    },

    /// Core-layer error.
    #[error("core error: {0}")]
    Core(#[from] OthelloError),

    /// Error from the opponent player.
    #[error("player error: {0}")]
    Player(#[from] PlayerError),

    /// Other errors.
    #[error("rl error: {0}")]
    Other(String),
}
