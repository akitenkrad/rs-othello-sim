//! [`Player`] trait と関連エラー型．

use othello_core::{Color, GameResult, GameState, Move};
use thiserror::Error;

/// プレイヤーが手の選択中に発生し得るエラー．
#[derive(Debug, Error)]
pub enum PlayerError {
    /// 標準入力やその他のソースからの読み取りに失敗した．
    #[error("input error: {0}")]
    Input(#[from] std::io::Error),

    /// 入力された文字列が座標として解釈できない．
    #[error("invalid coordinate: {input:?} ({reason})")]
    InvalidCoord {
        /// 入力された文字列．
        input: String,
        /// 詳細な失敗理由．
        reason: &'static str,
    },

    /// プレイヤーが合法手以外を返そうとした．
    #[error("player attempted an illegal move")]
    IllegalMove,

    /// EOF など入力が枯渇した．
    #[error("input exhausted before move was selected")]
    InputExhausted,

    /// その他．
    #[error("player error: {0}")]
    Other(String),
}

/// Othello プレイヤー戦略の trait．
///
/// `Send` 境界はバッチ並列実行 ( Phase 4 の `BatchRunner`) で使うために要求する．
pub trait Player: Send {
    /// プレイヤー名 ( ロギングや棋譜記録に利用される)．
    fn name(&self) -> &str;

    /// プレイヤーの色 ( 黒/白)．
    fn color(&self) -> Color;

    /// 現局面で次の手を選ぶ．
    ///
    /// パスしか取れない局面では，呼び出し側 ( engine) が `Move::Pass` を強制するため，
    /// このメソッドが Pass を返すのは「合法手があるのに Pass を返した」場合に限られる．
    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError>;

    /// ゲーム終了時に呼ばれる ( 学習系プレイヤーの後処理用)．
    fn on_game_end(&mut self, _final_state: &GameState, _result: GameResult) {}

    /// ゲーム開始時に呼ばれる ( 状態のリセット用)．
    fn reset(&mut self) {}
}
