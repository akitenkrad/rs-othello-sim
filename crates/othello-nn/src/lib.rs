//! # othello-nn
//!
//! NN-based evaluator crate added in Phase 6.4. Provides
//! [`NnEvaluator`], which implements both [`othello_player::Player`] and
//! [`othello_player::Evaluator`], together with policy/value evaluation
//! models ([`CandleModel`] / [`OnnxModel`]).
//!
//! ## Model I/O
//!
//! - Input: `f32` tensor of shape `(B, 3, H, W)`. The three channels
//!   are **own / opp / legal mask** in that order, following the same
//!   convention as [`othello_rl::Observation::Planes`].
//! - Output: policy logits `(B, H*W + 1)` (the trailing entry is the
//!   Pass logit) and a value scalar `(B,)` (win-rate estimate from the
//!   side to move's perspective, in tanh space $[-1, 1]$).
//!
//! ## Backends
//!
//! - [`CandleModel::random_init`] — builds a small randomly-initialized
//!   ResNet for testing.
//! - [`CandleModel::from_safetensors`] — loads trained weights from a
//!   Candle-native safetensors file.
//! - [`OnnxModel::from_path`] — loads an ONNX file and runs inference
//!   via `candle_onnx::simple_eval`. Useful for integrating
//!   AlphaZero-style models.
//!
//! ## CLI
//!
//! `othello-cli` can build players from PlayerSpec strings of the form
//! `nn:safetensors:PATH[,...]` or `nn:onnx:PATH[,...]`. To keep the
//! Candle dependency isolated, the spec is parsed in `othello-player`
//! while actual player construction happens in the `othello-cli`
//! wrapper.
//!
//! ## Example
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

/// Prelude that imports the commonly used types in one go.
pub mod prelude {
    pub use crate::{CandleModel, NnError, NnEvaluator, NnModel, OnnxModel, PolicyValue};
}
