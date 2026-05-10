//! Format-neutral game-record representation. [`GameRecord`] is the
//! shared internal representation used by JSON, GGF, WTHOR, etc.

use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Move};
use serde::{Deserialize, Serialize};

/// Current native JSON schema version.
pub const SCHEMA_VERSION: &str = "1.0";

/// A single game record (format-neutral).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameRecord {
    /// Schema version string.
    pub schema_version: String,
    /// Metadata.
    pub metadata: GameMetadata,
    /// Move sequence.
    pub moves: Vec<MoveEntry>,
}

impl GameRecord {
    /// Builds a record from `metadata` and `moves`, using the default
    /// `schema_version`.
    #[must_use]
    pub fn new(metadata: GameMetadata, moves: Vec<MoveEntry>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            metadata,
            moves,
        }
    }
}

/// Metadata for a game record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameMetadata {
    /// Game ID (typically a UUID v4).
    pub id: String,
    /// Start timestamp (with time zone).
    pub started_at: DateTime<FixedOffset>,
    /// End timestamp (`None` if the game is still in progress).
    pub ended_at: Option<DateTime<FixedOffset>>,
    /// Board size.
    pub board_size: BoardSize,
    /// Player information for black and white.
    pub players: PlayerPair,
    /// Game result (`None` if still in progress).
    pub result: Option<GameResultRecord>,
    /// Engine version string.
    pub engine_version: String,
}

/// Pair of black and white players.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerPair {
    /// Black player.
    pub black: PlayerInfo,
    /// White player.
    pub white: PlayerInfo,
}

/// Per-player information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    /// Player name (taken from `Player::name()`).
    pub name: String,
    /// Arbitrary parameters (e.g. seed, simulation count).
    pub params: serde_json::Value,
}

impl PlayerInfo {
    /// Builds a name-only `PlayerInfo` with empty parameters.
    #[must_use]
    pub fn just_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// Terminal result record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameResultRecord {
    /// Winner (`None` for a draw).
    pub winner: Option<Color>,
    /// Stone counts at the end of the game.
    pub score: Score,
}

/// Stone counts at the end of a game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Score {
    /// Black stone count.
    pub black: u32,
    /// White stone count.
    pub white: u32,
}

/// One move-entry record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveEntry {
    /// Move number (1-based).
    pub n: u32,
    /// Side that played the move.
    pub side: Color,
    /// The move (`Move::Place` or `Move::Pass`).
    #[serde(rename = "move")]
    pub r#move: Move,
    /// Timestamp of the move.
    pub ts: DateTime<FixedOffset>,
}
