//! NN model trait and inference result type.

use crate::error::NnError;
use candle_core::{Device, Tensor};
use othello_core::BoardSize;

/// Interface that an NN policy/value model must satisfy.
///
/// Implementations are expected to return **both** the policy head
/// (logits) and the value head (scalar) in a single forward pass.
/// Models with separated heads should call forward twice from the
/// caller side.
///
/// The `Send` bound is required so models can be used by the parallel
/// `othello_engine::BatchRunner`.
pub trait NnModel: Send {
    /// Runs inference.
    ///
    /// - `input`: `(B, 3, H, W)` `f32` tensor (own / opp / legal mask).
    /// - Returns `(policy_logits, value)`:
    ///   - `policy_logits` has shape `(B, H*W + 1)` (the last entry is
    ///     the Pass logit).
    ///   - `value` has shape `(B,)` in tanh space (a win-rate estimate
    ///     from the side to move's perspective in $[-1, 1]$).
    fn forward(&self, input: &Tensor) -> Result<(Tensor, Tensor), NnError>;

    /// Expected input board size. Used to validate the `input` shape.
    fn board_size(&self) -> BoardSize;

    /// Execution device (CPU / Metal / CUDA). Currently only CPU is
    /// targeted.
    fn device(&self) -> &Device;
}

/// NN output for a single position.
///
/// - `policy`: softmax-normalized probability distribution (sums to
///   1.0). Length `H*W + 1`; the last entry is Pass.
/// - `value`: win-rate estimate in tanh space $[-1, 1]$ from the side
///   to move's perspective (1 is winning).
#[derive(Debug, Clone)]
pub struct PolicyValue {
    /// Probability vector after softmax.
    pub policy: Vec<f32>,
    /// Value in tanh space.
    pub value: f32,
}
