//! # othello-player
//!
//! [`Player`] trait and standard implementations of Othello playing
//! strategies.
//!
//! ## Provided implementations
//!
//! - [`HumanPlayer`] — Reads coordinate strings from stdin (or any
//!   `Read`).
//! - [`RandomPlayer`] — Samples uniformly from legal moves using
//!   `ChaCha8Rng`.
//! - [`GreedyPlayer`] — Picks the move that maximizes the player's stone
//!   count after placing.
//! - [`MctsPlayer`] — UCT-based MCTS (Phase 4).
//!
//! ## Example
//!
//! ```
//! use othello_core::prelude::*;
//! use othello_player::{Player, RandomPlayer};
//!
//! let mut p = RandomPlayer::with_seed(Color::Black, 42);
//! let s = GameState::standard_8x8();
//! let mv = p.select_move(&s).unwrap();
//! assert!(matches!(mv, Move::Place(_)));
//! ```

pub mod greedy;
pub mod human;
pub mod mcts;
pub mod player_spec;
pub mod random;
pub mod traits;

pub use greedy::GreedyPlayer;
pub use human::HumanPlayer;
pub use mcts::{MctsConfig, MctsPlayer};
pub use player_spec::{NnBackend, NnSpec, PlayerSpec, parse_player_spec, spec_name, spec_params};
pub use random::RandomPlayer;
pub use traits::{Evaluator, Player, PlayerError};

pub mod external;
pub use external::{ExternalEngineConfig, ExternalEnginePlayer, Protocol};

/// Prelude that imports the commonly used types in one go.
pub mod prelude {
    pub use crate::{
        Evaluator, GreedyPlayer, HumanPlayer, MctsConfig, MctsPlayer, Player, PlayerError,
        PlayerSpec, RandomPlayer,
    };
}
