//! [`RlError`]: 強化学習環境のエラー型．

use othello_core::OthelloError;
use othello_player::PlayerError;
use thiserror::Error;

/// 強化学習環境で発生し得るエラー．
#[derive(Debug, Error)]
pub enum RlError {
    /// `step` に渡された Action が現局面で合法でない．
    #[error("illegal action: {action} (legal_count={legal_count})")]
    IllegalAction {
        /// Action インデックス．
        action: u32,
        /// 当該局面での合法手数．
        legal_count: u32,
    },

    /// Action 値が範囲外 ( 0..space_size 外)．
    #[error("action {action} out of range (max {max})")]
    OutOfRange {
        /// 与えられた Action．
        action: u32,
        /// 許容上限 ( 含まない)．
        max: u32,
    },

    /// 手番がエージェントでない局面で `step` が呼ばれた ( マルチエージェント環境で軽い safety)．
    #[error("not your turn: side_to_move={side:?}, agent={agent:?}")]
    NotYourTurn {
        /// 現手番．
        side: othello_core::Color,
        /// エージェント色．
        agent: othello_core::Color,
    },

    /// コア層のエラー．
    #[error("core error: {0}")]
    Core(#[from] OthelloError),

    /// プレイヤー ( opponent) 側のエラー．
    #[error("player error: {0}")]
    Player(#[from] PlayerError),

    /// その他．
    #[error("rl error: {0}")]
    Other(String),
}
