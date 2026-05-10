//! Errors raised by the NN evaluator.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that the NN evaluator can return.
#[derive(Debug, Error)]
pub enum NnError {
    /// Weight-file I/O failure (e.g. file not found).
    #[error("failed to load weights from {path}: {source}")]
    Io {
        /// Path that failed to load.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Candle internal error (e.g. shape mismatch).
    #[error("candle error: {0}")]
    Candle(String),

    /// ONNX parse failure.
    #[error("onnx error: {0}")]
    Onnx(String),

    /// Input tensor shape differs from what was expected.
    #[error("input shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        /// Expected shape.
        expected: Vec<usize>,
        /// Actual shape.
        actual: Vec<usize>,
    },

    /// Unexpected output shape.
    #[error("output shape mismatch: {0}")]
    OutputShape(String),

    /// Invalid model configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<candle_core::Error> for NnError {
    fn from(e: candle_core::Error) -> Self {
        NnError::Candle(format!("{e}"))
    }
}
