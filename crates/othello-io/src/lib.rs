//! # othello-io
//!
//! Othello 棋譜の入出力．形式中立な [`GameRecord`] と，それに対する
//! [`GameRecordReader`] / [`GameRecordWriter`] trait で抽象化する．
//!
//! ## サポート形式
//!
//! - [`json`] — 自前 JSON ( タイムスタンプ・メタデータ込み)．設計書 §4.1 準拠．
//! - [`ggf`] — Generic Game Format ( Othello サブセット)．設計書 §4.3 準拠．

pub mod error;
pub mod ggf;
pub mod json;
pub mod record;
pub mod traits;

pub use error::IoError;
pub use ggf::{GgfReader, GgfWriter};
pub use json::{JsonReader, JsonWriter};
pub use record::{
    GameMetadata, GameRecord, GameResultRecord, MoveEntry, PlayerInfo, PlayerPair, SCHEMA_VERSION,
    Score,
};
pub use traits::{GameRecordReader, GameRecordWriter};

/// よく使う型を一括で導入するための prelude．
pub mod prelude {
    pub use crate::{
        GameMetadata, GameRecord, GameRecordReader, GameRecordWriter, GameResultRecord, GgfReader,
        GgfWriter, IoError, JsonReader, JsonWriter, MoveEntry, PlayerInfo, PlayerPair, Score,
    };
}
