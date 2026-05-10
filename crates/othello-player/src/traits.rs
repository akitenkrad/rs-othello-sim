//! [`Player`] trait and the associated error types.

use othello_core::{Color, GameResult, GameState, Move};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that may occur while a player chooses a move.
#[derive(Debug, Error)]
pub enum PlayerError {
    /// Failed to read from stdin or another source.
    #[error("input error: {0}")]
    Input(#[from] std::io::Error),

    /// The input string cannot be parsed as a coordinate.
    #[error("invalid coordinate: {input:?} ({reason})")]
    InvalidCoord {
        /// The original input string.
        input: String,
        /// Detailed reason for the failure.
        reason: &'static str,
    },

    /// The player attempted to return an illegal move.
    #[error("player attempted an illegal move")]
    IllegalMove,

    /// Input was exhausted (e.g. EOF) before a move could be selected.
    #[error("input exhausted before move was selected")]
    InputExhausted,

    /// Other errors.
    #[error("player error: {0}")]
    Other(String),
}

/// Trait for Othello playing strategies.
///
/// The `Send` bound is required so that players can be used by the
/// parallel batch runner (`BatchRunner`, Phase 4).
pub trait Player: Send {
    /// Player name (used for logging and game records).
    fn name(&self) -> &str;

    /// Player color (black/white).
    fn color(&self) -> Color;

    /// Selects the next move from the given state.
    ///
    /// When only `Pass` is legal, the engine substitutes `Move::Pass` on
    /// the player's behalf, so the only valid scenario in which this
    /// method returns `Pass` is when the player erroneously passes while
    /// legal moves exist.
    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError>;

    /// Hook invoked at the end of a game (e.g. for learning post-processing).
    fn on_game_end(&mut self, _final_state: &GameState, _result: GameResult) {}

    /// Hook invoked at the start of a game (e.g. to reset internal state).
    fn reset(&mut self) {}

    /// Returns a mutable reference to the evaluator if the player
    /// implements [`Evaluator`].
    ///
    /// Defaults to `None`. Players such as `MctsPlayer` should override
    /// this to expose themselves. Used to peek at evaluation values from
    /// the TUI Observe overlay or other external consumers.
    fn evaluator(&mut self) -> Option<&mut dyn Evaluator> {
        None
    }
}

/// Trait that returns evaluation scores for each legal move (e.g.
/// normalized visit counts or win-rate estimates).
///
/// Helper trait used to peek at MCTS visit counts or NN policy outputs
/// from the TUI Observe mode and the CLI. Players that do not support
/// evaluation can either skip implementing it or have
/// [`Evaluator::evaluate`] return `None`.
///
/// Scores follow the **higher-is-better** convention (e.g. win-rate
/// estimates in `0.0..=1.0`). The caller may emphasize the maximum value
/// when rendering.
///
/// ## Example
///
/// ```ignore
/// use othello_core::{Move, GameState};
/// use othello_player::{Evaluator, MctsConfig, MctsPlayer, Player, traits::Evaluator as _};
/// use std::collections::HashMap;
///
/// let mut p = MctsPlayer::new(othello_core::Color::Black, MctsConfig::new(100));
/// let s = GameState::standard_8x8();
/// let scores: Option<HashMap<Move, f32>> = p.evaluate(&s);
/// ```
pub trait Evaluator: Send {
    /// Returns a per-legal-move score map (keys are legal `Move`s).
    ///
    /// May update internal state as a side effect (e.g. running one
    /// additional MCTS rollout). Players that do not support evaluation
    /// return `None`.
    fn evaluate(&mut self, state: &GameState) -> Option<HashMap<Move, f32>>;
}
