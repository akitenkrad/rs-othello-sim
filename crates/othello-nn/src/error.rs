//! NN 関連のエラー型．

use std::path::PathBuf;
use thiserror::Error;

/// NN 評価器が返し得るエラー．
#[derive(Debug, Error)]
pub enum NnError {
    /// 重みファイルの I/O 失敗 ( ファイル不在等)．
    #[error("failed to load weights from {path}: {source}")]
    Io {
        /// 失敗したパス．
        path: PathBuf,
        /// 元の I/O エラー．
        #[source]
        source: std::io::Error,
    },

    /// Candle 内部のエラー ( shape 不整合等)．
    #[error("candle error: {0}")]
    Candle(String),

    /// ONNX 解析失敗．
    #[error("onnx error: {0}")]
    Onnx(String),

    /// 期待していた入力 shape と異なる．
    #[error("input shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        /// 期待 shape．
        expected: Vec<usize>,
        /// 実際の shape．
        actual: Vec<usize>,
    },

    /// 出力 shape が想定外．
    #[error("output shape mismatch: {0}")]
    OutputShape(String),

    /// モデル設定の不整合．
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<candle_core::Error> for NnError {
    fn from(e: candle_core::Error) -> Self {
        NnError::Candle(format!("{e}"))
    }
}
