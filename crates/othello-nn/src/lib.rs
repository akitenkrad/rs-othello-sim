//! # othello-nn
//!
//! Phase 6.4 で追加された **NN ベース評価器** クレート．[`othello_player::Player`] と
//! [`othello_player::Evaluator`] の双方を実装する [`NnEvaluator`] と，policy ヘッド +
//! value ヘッドを持つ評価モデル ( [`CandleModel`] / [`OnnxModel`]) を提供する．
//!
//! ## モデルの入出力
//!
//! - 入力: `(B, 3, H, W)` の `f32` テンソル．3 channel は順に **own / opp / legal mask**
//!   を表し，[`othello_rl::Observation::Planes`] と同じ規約に従う．
//! - 出力: policy logits `(B, H*W + 1)` ( 末尾は Pass) と value scalar `(B,)` ( 自分視点
//!   の勝率推定，tanh 空間 $[-1, 1]$)．
//!
//! ## バックエンド
//!
//! - [`CandleModel::random_init`] — テスト用にランダム初期化された小型 ResNet を構築する．
//! - [`CandleModel::from_safetensors`] — Candle ネイティブの safetensors ファイルから
//!   学習済重みを読み込む．
//! - [`OnnxModel::from_path`] — ONNX ファイルを読み込み，`candle_onnx::simple_eval` で
//!   推論する．AlphaZero 系のモデル統合用．
//!
//! ## CLI
//!
//! `nn:safetensors:PATH[,...]` / `nn:onnx:PATH[,...]` 形式の PlayerSpec を `othello-cli`
//! 側で構築できる ( [`othello_player`] からは Candle 依存を分離するため，spec のパースは
//! `othello-player` で行うが，実際の Player 構築は `othello-cli` の wrapper 関数で
//! 行う)．
//!
//! ## 例
//!
//! ```no_run
//! use othello_nn::{CandleModel, NnEvaluator};
//! use othello_core::{BoardSize, Color, GameState};
//! use othello_player::Player;
//! use candle_core::Device;
//!
//! let model = CandleModel::random_init(BoardSize::STANDARD, 42, Device::Cpu).unwrap();
//! let mut player = NnEvaluator::new(Color::Black, model)
//!     .with_seed(7)
//!     .deterministic();
//! let mv = player.select_move(&GameState::standard_8x8()).unwrap();
//! println!("nn picked {:?}", mv);
//! ```

pub mod candle_model;
pub mod error;
pub mod evaluator;
pub mod input;
pub mod model;
pub mod onnx_model;

pub use candle_model::CandleModel;
pub use error::NnError;
pub use evaluator::NnEvaluator;
pub use input::state_to_tensor;
pub use model::{NnModel, PolicyValue};
pub use onnx_model::OnnxModel;

/// よく使う型を一括で導入するための prelude．
pub mod prelude {
    pub use crate::{CandleModel, NnError, NnEvaluator, NnModel, OnnxModel, PolicyValue};
}
