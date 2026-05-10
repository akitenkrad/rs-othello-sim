//! [`OnnxModel`]: load a trained model via ONNX and run inference.
//!
//! Inference uses the `simple_eval` function from `candle-onnx`. The
//! input/output tensor names can be overridden via
//! [`OnnxModel::with_io_names`] (defaults: `input` / `policy` /
//! `value`).
//!
//! ## Expected I/O schema
//!
//! - One input
//!   - Name: `input` (default).
//!   - Shape: `(B, 3, H, W)`.
//! - Two outputs
//!   - `policy`: `(B, H*W + 1)` policy logits (before softmax).
//!   - `value`: `(B,)` or `(B, 1)` value scalar (already tanh-applied).
//!
//! AlphaZero-style exports conventionally use `policy` / `value` as
//! the output names.

use crate::error::NnError;
use crate::model::NnModel;
use candle_core::{Device, Tensor};
use candle_onnx::onnx::ModelProto;
use othello_core::BoardSize;
use std::collections::HashMap;
use std::path::Path;

/// ONNX-format model.
pub struct OnnxModel {
    proto: ModelProto,
    board_size: BoardSize,
    device: Device,
    input_name: String,
    policy_name: String,
    value_name: String,
}

impl OnnxModel {
    /// Loads an ONNX file using the default I/O names (`input` /
    /// `policy` / `value`).
    pub fn from_path<P: AsRef<Path>>(path: P, board_size: BoardSize) -> Result<Self, NnError> {
        Self::from_path_with_io(path, board_size, "input", "policy", "value")
    }

    /// Loads an ONNX file with explicit I/O names. Use this when the
    /// model was exported with custom names.
    pub fn from_path_with_io<P: AsRef<Path>>(
        path: P,
        board_size: BoardSize,
        input_name: &str,
        policy_name: &str,
        value_name: &str,
    ) -> Result<Self, NnError> {
        let p = path.as_ref();
        if !p.exists() {
            return Err(NnError::Io {
                path: p.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "onnx file not found"),
            });
        }
        let proto = candle_onnx::read_file(p).map_err(|e| NnError::Onnx(format!("{e}")))?;
        Ok(Self {
            proto,
            board_size,
            device: Device::Cpu,
            input_name: input_name.to_string(),
            policy_name: policy_name.to_string(),
            value_name: value_name.to_string(),
        })
    }

    /// Replaces the I/O names after construction.
    #[must_use]
    pub fn with_io_names(mut self, input: &str, policy: &str, value: &str) -> Self {
        self.input_name = input.to_string();
        self.policy_name = policy.to_string();
        self.value_name = value.to_string();
        self
    }
}

impl NnModel for OnnxModel {
    fn forward(&self, input: &Tensor) -> Result<(Tensor, Tensor), NnError> {
        let dims = input.shape().dims().to_vec();
        let expected_h = self.board_size.rows as usize;
        let expected_w = self.board_size.cols as usize;
        if dims.len() != 4 || dims[1] != 3 || dims[2] != expected_h || dims[3] != expected_w {
            return Err(NnError::ShapeMismatch {
                expected: vec![0, 3, expected_h, expected_w],
                actual: dims,
            });
        }

        let mut inputs: HashMap<String, Tensor> = HashMap::new();
        inputs.insert(self.input_name.clone(), input.clone());
        let outputs = candle_onnx::simple_eval(&self.proto, inputs)
            .map_err(|e| NnError::Onnx(format!("simple_eval: {e}")))?;
        let policy = outputs.get(&self.policy_name).ok_or_else(|| {
            NnError::OutputShape(format!(
                "policy output {:?} not found in onnx model",
                self.policy_name
            ))
        })?;
        let value = outputs.get(&self.value_name).ok_or_else(|| {
            NnError::OutputShape(format!(
                "value output {:?} not found in onnx model",
                self.value_name
            ))
        })?;
        // value は (B, 1) でも (B,) でもよい．flatten_all して (B,) に揃える ( B=1 前提)．
        let value = if value.shape().dims().len() > 1 {
            value.flatten_all()?
        } else {
            value.clone()
        };
        Ok((policy.clone(), value))
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
    fn from_path_missing_file() {
        let r = OnnxModel::from_path("/tmp/__nonexistent_othello.onnx", BoardSize::STANDARD);
        assert!(matches!(r, Err(NnError::Io { .. })));
    }

    #[test]
    fn from_path_with_io_missing_file() {
        let r = OnnxModel::from_path_with_io(
            "/tmp/__nonexistent_othello.onnx",
            BoardSize::STANDARD,
            "x",
            "p",
            "v",
        );
        assert!(matches!(r, Err(NnError::Io { .. })));
    }
}
