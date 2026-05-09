//! [`Player`] trait と関連エラー型．

use othello_core::{Color, GameResult, GameState, Move};
use std::collections::HashMap;
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

    /// プレイヤーが [`Evaluator`] を実装している場合，その可変参照を返す．
    ///
    /// デフォルト実装は `None`．`MctsPlayer` 等は override してこのメソッドから自身を返す．
    /// TUI Observe モードの evaluator overlay 表示など，外部から評価値を覗く用途に使う．
    fn evaluator(&mut self) -> Option<&mut dyn Evaluator> {
        None
    }
}

/// 各合法手に対する評価値 ( 勝率・visit 数を正規化した値など) を返す trait．
///
/// MCTS の visit count や NN の policy 値を TUI Observe モードや CLI から覗くために用意した
/// 補助 trait．対応していないプレイヤーは何も実装しないか，[`Evaluator::evaluate`] が
/// `None` を返せばよい．
///
/// 値は `0.0..=1.0` の勝率推定など，**大きいほど良い** 値とする．呼び出し側は表示時に
/// 最大値を強調するなどの用途に使う．
///
/// ## 例
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
    /// 各合法手の評価値を返す ( キーは合法手の `Move`)．
    ///
    /// 呼び出しによって内部状態を更新してよい ( 例: MCTS の木を 1 手分だけ走らせる)．
    /// 評価未対応のプレイヤーは `None` を返す．
    fn evaluate(&mut self, state: &GameState) -> Option<HashMap<Move, f32>>;
}
