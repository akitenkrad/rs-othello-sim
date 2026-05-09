//! # othello-engine
//!
//! Othello のゲーム実行ループと履歴管理．主要型:
//!
//! - [`GameEngine`] — `Player` を 2 つ受け取って 1 局を進行する．
//! - [`GameHistory`] — 全手のスナップショットを保持し，後から再生可能にする．
//! - [`Replayer`] — `GameHistory` 上で前後移動・任意手数ジャンプを行う．

pub mod engine;
pub mod history;
pub mod replayer;

pub use engine::{EngineConfig, EngineError, GameEngine};
pub use history::GameHistory;
pub use replayer::Replayer;

/// よく使う型を一括で導入するための prelude．
pub mod prelude {
    pub use crate::{EngineConfig, EngineError, GameEngine, GameHistory, Replayer};
}
