//! # othello-io
//!
//! Othello 棋譜の入出力．形式中立な [`GameRecord`] と，それに対する
//! [`GameRecordReader`] / [`GameRecordWriter`] trait で抽象化する．
//!
//! ## サポート形式
//!
//! - [`json`] — 自前 JSON ( タイムスタンプ・メタデータ込み)．設計書 §4.1 準拠．
//! - [`ggf`] — Generic Game Format ( Othello サブセット)．設計書 §4.3 準拠．
//! - [`wthor`] — WTHOR (.wtb) バイナリ形式．読込のみ．設計書 §4.4 準拠．
//! - [`jsonl_log`] — 進行イベント JSONL ( 書込のみ)．設計書 §4.2 準拠．

pub mod error;
pub mod ggf;
pub mod json;
pub mod jsonl_log;
pub mod record;
pub mod traits;
pub mod wthor;

pub use error::IoError;
pub use ggf::{GgfReader, GgfWriter};
pub use json::{JsonReader, JsonWriter};
pub use jsonl_log::{
    GameEndEvent, GameStartEvent, JsonlLogger, LogEvent, MoveEvent, MoveSerial, PassEvent,
    PlayerNames, Stones,
};
pub use record::{
    GameMetadata, GameRecord, GameResultRecord, MoveEntry, PlayerInfo, PlayerPair, SCHEMA_VERSION,
    Score,
};
pub use traits::{GameRecordReader, GameRecordWriter};
pub use wthor::{WthorHeader, WthorReader};

/// よく使う型を一括で導入するための prelude．
pub mod prelude {
    pub use crate::{
        GameEndEvent, GameMetadata, GameRecord, GameRecordReader, GameRecordWriter,
        GameResultRecord, GameStartEvent, GgfReader, GgfWriter, IoError, JsonReader, JsonWriter,
        JsonlLogger, LogEvent, MoveEntry, MoveEvent, MoveSerial, PassEvent, PlayerInfo,
        PlayerNames, PlayerPair, Score, Stones, WthorHeader, WthorReader,
    };
}
