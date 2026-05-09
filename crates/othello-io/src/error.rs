//! [`IoError`]: othello-io の入出力エラー．

use thiserror::Error;

/// 棋譜の読み書き中に発生するエラー．
#[derive(Debug, Error)]
pub enum IoError {
    /// 標準入出力エラー．
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON シリアライズ/デシリアライズエラー．
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// 棋譜フォーマットの解析失敗．
    #[error("parse error: {0}")]
    Parse(String),

    /// スキーマバージョン不一致．
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),

    /// その他．
    #[error("io error: {0}")]
    Other(String),
}
