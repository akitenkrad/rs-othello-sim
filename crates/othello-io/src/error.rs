//! [`IoError`]: I/O error for `othello-io`.

use thiserror::Error;

/// Error raised while reading or writing game records.
#[derive(Debug, Error)]
pub enum IoError {
    /// Standard I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to parse the game-record format.
    #[error("parse error: {0}")]
    Parse(String),

    /// Schema version mismatch.
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),

    /// Other errors.
    #[error("io error: {0}")]
    Other(String),
}
