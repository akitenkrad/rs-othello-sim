//! # othello-engine
//!
//! Game-execution loop and history management for Othello. Key types:
//!
//! - [`GameEngine`] — Takes two `Player`s and drives a full game.
//! - [`GameHistory`] — Stores per-move snapshots so a game can be replayed.
//! - [`Replayer`] — Steps through a `GameHistory` (forward, backward,
//!   jump-to).

pub mod batch;
pub mod engine;
pub mod history;
pub mod replayer;

pub use batch::{BatchConfig, BatchError, BatchResult, BatchRunner, GameSummary, ProgressCallback};
pub use engine::{EngineConfig, EngineError, GameEngine};
pub use history::GameHistory;
pub use replayer::Replayer;

/// Prelude that imports the commonly used types in one go.
pub mod prelude {
    pub use crate::{
        BatchConfig, BatchError, BatchResult, BatchRunner, EngineConfig, EngineError, GameEngine,
        GameHistory, GameSummary, Replayer,
    };
}
