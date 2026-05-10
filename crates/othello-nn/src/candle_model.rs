//! Candle-based ResNet-style policy/value model.
//!
//! ## Architecture
//!
//! ```text
//! input (B, 3, H, W)
//!   -> conv 3x3 (3 -> CHANNELS) -> BN -> ReLU       (stem)
//!   -> 2 x ResidualBlock (CHANNELS, CHANNELS)
//!   |- policy head: conv 1x1 (CHANNELS -> 2) -> BN -> ReLU -> Linear(2*H*W -> H*W + 1)
//!   |- value head:  conv 1x1 (CHANNELS -> 1) -> BN -> ReLU -> Linear(H*W -> 64) -> ReLU -> Linear(64 -> 1) -> tanh
//! ```
//!
//! Phase 6.4 does not ship a trained model. The crate only exposes
//! [`CandleModel::random_init`] (random-initialized weights) and
//! [`CandleModel::from_safetensors`] (load from a safetensors file).

use crate::error::NnError;
use crate::model::NnModel;
use candle_core::{DType, Device, Module, ModuleT, Tensor};
use candle_nn::{
    BatchNorm, BatchNormConfig, Conv2d, Conv2dConfig, Linear, VarBuilder, VarMap, batch_norm,
    conv2d, linear,
};
use othello_core::BoardSize;
use std::path::Path;

const CHANNELS: usize = 32;
const NUM_RES_BLOCKS: usize = 2;
const VALUE_HIDDEN: usize = 64;

/// One residual block (conv-BN-ReLU-conv-BN + skip -> ReLU).
struct ResidualBlock {
    conv1: Conv2d,
    bn1: BatchNorm,
    conv2: Conv2d,
    bn2: BatchNorm,
}

impl ResidualBlock {
    fn new(channels: usize, vb: VarBuilder) -> Result<Self, NnError> {
        let cfg = Conv2dConfig {
            padding: 1,
            ..Default::default()
        };
        let conv1 = conv2d(channels, channels, 3, cfg, vb.pp("conv1"))?;
        let bn1 = batch_norm(channels, BatchNormConfig::default(), vb.pp("bn1"))?;
        let conv2 = conv2d(channels, channels, 3, cfg, vb.pp("conv2"))?;
        let bn2 = batch_norm(channels, BatchNormConfig::default(), vb.pp("bn2"))?;
        Ok(Self {
            conv1,
            bn1,
            conv2,
            bn2,
        })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor, NnError> {
        let y = self.conv1.forward(x)?;
        let y = self.bn1.forward_t(&y, false)?;
        let y = y.relu()?;
        let y = self.conv2.forward(&y)?;
        let y = self.bn2.forward_t(&y, false)?;
        let y = y.add(x)?;
        let y = y.relu()?;
        Ok(y)
    }
}

/// ResNet-style policy/value model implemented in Candle.
pub struct CandleModel {
    stem_conv: Conv2d,
    stem_bn: BatchNorm,
    blocks: Vec<ResidualBlock>,
    policy_conv: Conv2d,
    policy_bn: BatchNorm,
    policy_linear: Linear,
    value_conv: Conv2d,
    value_bn: BatchNorm,
    value_linear1: Linear,
    value_linear2: Linear,
    board_size: BoardSize,
    device: Device,
    /// Handle that retains the weights for inference. Held purely to
    /// defer weight deallocation until `Drop`.
    _varmap: VarMap,
}

impl CandleModel {
    /// Initializes every weight randomly via Candle's default
    /// initializer (intended for tests).
    ///
    /// `seed` is reserved for a future deterministic-init path.
    /// Currently `VarMap::all_vars` in candle uses **non-deterministic**
    /// initialization, so the parameter is kept in the API for
    /// compatibility but is not used internally.
    pub fn random_init(board_size: BoardSize, _seed: u64, device: Device) -> Result<Self, NnError> {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        Self::build(board_size, device, varmap, vb)
    }

    /// Loads weights from a safetensors file.
    ///
    /// The saved tensor names must match the `pp(...)` hierarchy used
    /// here, e.g. `stem_conv.weight`,
    /// `stem_bn.{weight,bias,running_mean,running_var}`,
    /// `block_{i}.conv1.weight`, etc.
    pub fn from_safetensors<P: AsRef<Path>>(
        path: P,
        board_size: BoardSize,
        device: Device,
    ) -> Result<Self, NnError> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(NnError::Io {
                path: p.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "safetensors file not found",
                ),
            });
        }
        let varmap = VarMap::new();
        let vb_load = unsafe {
            VarBuilder::from_mmaped_safetensors(&[p], DType::F32, &device).map_err(|e| {
                NnError::Io {
                    path: p.to_path_buf(),
                    source: std::io::Error::other(format!("{e}")),
                }
            })?
        };
        Self::build(board_size, device, varmap, vb_load)
    }

    fn build(
        board_size: BoardSize,
        device: Device,
        varmap: VarMap,
        vb: VarBuilder,
    ) -> Result<Self, NnError> {
        let conv_cfg = Conv2dConfig {
            padding: 1,
            ..Default::default()
        };
        let conv1_cfg = Conv2dConfig::default(); // 1x1 conv
        let stem_conv = conv2d(3, CHANNELS, 3, conv_cfg, vb.pp("stem_conv"))?;
        let stem_bn = batch_norm(CHANNELS, BatchNormConfig::default(), vb.pp("stem_bn"))?;
        let mut blocks = Vec::with_capacity(NUM_RES_BLOCKS);
        for i in 0..NUM_RES_BLOCKS {
            let block = ResidualBlock::new(CHANNELS, vb.pp(format!("block_{i}")))?;
            blocks.push(block);
        }
        let h = board_size.rows as usize;
        let w = board_size.cols as usize;
        let policy_conv = conv2d(CHANNELS, 2, 1, conv1_cfg, vb.pp("policy_conv"))?;
        let policy_bn = batch_norm(2, BatchNormConfig::default(), vb.pp("policy_bn"))?;
        let policy_linear = linear(2 * h * w, h * w + 1, vb.pp("policy_linear"))?;
        let value_conv = conv2d(CHANNELS, 1, 1, conv1_cfg, vb.pp("value_conv"))?;
        let value_bn = batch_norm(1, BatchNormConfig::default(), vb.pp("value_bn"))?;
        let value_linear1 = linear(h * w, VALUE_HIDDEN, vb.pp("value_linear1"))?;
        let value_linear2 = linear(VALUE_HIDDEN, 1, vb.pp("value_linear2"))?;
        Ok(Self {
            stem_conv,
            stem_bn,
            blocks,
            policy_conv,
            policy_bn,
            policy_linear,
            value_conv,
            value_bn,
            value_linear1,
            value_linear2,
            board_size,
            device,
            _varmap: varmap,
        })
    }
}

impl NnModel for CandleModel {
    fn forward(&self, input: &Tensor) -> Result<(Tensor, Tensor), NnError> {
        // 入力 shape チェック (B, 3, H, W)
        let dims = input.shape().dims().to_vec();
        let expected_h = self.board_size.rows as usize;
        let expected_w = self.board_size.cols as usize;
        if dims.len() != 4 || dims[1] != 3 || dims[2] != expected_h || dims[3] != expected_w {
            return Err(NnError::ShapeMismatch {
                expected: vec![0, 3, expected_h, expected_w],
                actual: dims,
            });
        }

        let x = self.stem_conv.forward(input)?;
        let x = self.stem_bn.forward_t(&x, false)?;
        let mut x = x.relu()?;
        for block in &self.blocks {
            x = block.forward(&x)?;
        }
        // Policy head
        let p = self.policy_conv.forward(&x)?;
        let p = self.policy_bn.forward_t(&p, false)?;
        let p = p.relu()?;
        let p = p.flatten_from(1)?;
        let policy_logits = self.policy_linear.forward(&p)?;

        // Value head
        let v = self.value_conv.forward(&x)?;
        let v = self.value_bn.forward_t(&v, false)?;
        let v = v.relu()?;
        let v = v.flatten_from(1)?;
        let v = self.value_linear1.forward(&v)?;
        let v = v.relu()?;
        let v = self.value_linear2.forward(&v)?;
        let v = v.tanh()?;
        // (B, 1) → (B,)
        let value = v.flatten_all()?;

        Ok((policy_logits, value))
    }

    fn board_size(&self) -> BoardSize {
        self.board_size
    }

    fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_init_8x8_forward_shape() {
        let m = CandleModel::random_init(BoardSize::STANDARD, 7, Device::Cpu).unwrap();
        let input = Tensor::zeros((1, 3, 8, 8), DType::F32, &Device::Cpu).unwrap();
        let (p, v) = m.forward(&input).unwrap();
        assert_eq!(p.shape().dims(), &[1, 65]);
        assert_eq!(v.shape().dims(), &[1]);
    }

    #[test]
    fn random_init_4x4_forward_shape() {
        let m = CandleModel::random_init(BoardSize::square(4), 1, Device::Cpu).unwrap();
        let input = Tensor::zeros((1, 3, 4, 4), DType::F32, &Device::Cpu).unwrap();
        let (p, v) = m.forward(&input).unwrap();
        assert_eq!(p.shape().dims(), &[1, 17]);
        assert_eq!(v.shape().dims(), &[1]);
    }

    #[test]
    fn forward_rejects_wrong_shape() {
        let m = CandleModel::random_init(BoardSize::STANDARD, 0, Device::Cpu).unwrap();
        let input = Tensor::zeros((1, 3, 4, 4), DType::F32, &Device::Cpu).unwrap();
        let r = m.forward(&input);
        assert!(matches!(r, Err(NnError::ShapeMismatch { .. })));
    }

    #[test]
    fn from_safetensors_missing_file() {
        let r = CandleModel::from_safetensors(
            "/tmp/__nonexistent_othello_nn.safetensors",
            BoardSize::STANDARD,
            Device::Cpu,
        );
        assert!(matches!(r, Err(NnError::Io { .. })));
    }
}
