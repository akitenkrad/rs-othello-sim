//! # othello-player
//!
//! Othello プレイヤー戦略の [`Player`] trait と標準実装を提供する．
//!
//! ## 提供実装
//!
//! - [`HumanPlayer`] — 標準入力 ( または任意の `Read`) から座標文字列を読み取る
//! - [`RandomPlayer`] — `ChaCha8Rng` で合法手から一様サンプル
//! - [`GreedyPlayer`] — 着手後の自分の石数を最大化する手を選ぶ
//! - [`MctsPlayer`] — UCT に基づく MCTS ( Phase 4)
//!
//! ## 例
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
pub use player_spec::{PlayerSpec, parse_player_spec, spec_name, spec_params};
pub use random::RandomPlayer;
pub use traits::{Player, PlayerError};

/// よく使う型を一括で導入するための prelude．
pub mod prelude {
    pub use crate::{
        GreedyPlayer, HumanPlayer, MctsConfig, MctsPlayer, Player, PlayerError, PlayerSpec,
        RandomPlayer,
    };
}
