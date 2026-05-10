//! # othello-io
//!
//! Game-record I/O for Othello. Abstracts over formats with the
//! format-neutral [`GameRecord`] together with the [`GameRecordReader`]
//! and [`GameRecordWriter`] traits.
//!
//! ## Supported formats
//!
//! - [`json`] — Native JSON (with timestamps and metadata). Conforms to
//!   §4.1 of the design document.
//! - [`ggf`] — Generic Game Format (Othello subset). Conforms to §4.3
//!   of the design document.
//! - [`wthor`] — WTHOR (`.wtb`) binary format (read-only). Conforms to
//!   §4.4 of the design document.
//! - [`jsonl_log`] — Per-event JSONL log (write-only). Conforms to §4.2
//!   of the design document.

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

/// Prelude that imports the commonly used types in one go.
pub mod prelude {
    pub use crate::{
        GameEndEvent, GameMetadata, GameRecord, GameRecordReader, GameRecordWriter,
        GameResultRecord, GameStartEvent, GgfReader, GgfWriter, IoError, JsonReader, JsonWriter,
        JsonlLogger, LogEvent, MoveEntry, MoveEvent, MoveSerial, PassEvent, PlayerInfo,
        PlayerNames, PlayerPair, Score, Stones, WthorHeader, WthorReader,
    };
}
