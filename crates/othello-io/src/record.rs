//! 形式中立な棋譜表現．[`GameRecord`] が JSON / GGF / WTHOR 等の中立的な内部表現となる．

use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Move};
use serde::{Deserialize, Serialize};

/// 現行の自前 JSON スキーマバージョン．
pub const SCHEMA_VERSION: &str = "1.0";

/// 1 局の棋譜 ( 形式中立)．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameRecord {
    /// スキーマバージョン文字列．
    pub schema_version: String,
    /// メタデータ．
    pub metadata: GameMetadata,
    /// 着手列．
    pub moves: Vec<MoveEntry>,
}

impl GameRecord {
    /// `metadata` と `moves` から `schema_version` 既定値で構築する．
    #[must_use]
    pub fn new(metadata: GameMetadata, moves: Vec<MoveEntry>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            metadata,
            moves,
        }
    }
}

/// 棋譜のメタデータ．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameMetadata {
    /// ゲーム ID ( UUID v4 想定)．
    pub id: String,
    /// 開始時刻 ( タイムゾーン付き)．
    pub started_at: DateTime<FixedOffset>,
    /// 終了時刻 ( 進行中なら `None`)．
    pub ended_at: Option<DateTime<FixedOffset>>,
    /// 盤面サイズ．
    pub board_size: BoardSize,
    /// 黒白プレイヤー情報．
    pub players: PlayerPair,
    /// 結果 ( 進行中なら `None`)．
    pub result: Option<GameResultRecord>,
    /// エンジンバージョン文字列．
    pub engine_version: String,
}

/// 黒白プレイヤーの組．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerPair {
    /// 黒プレイヤー．
    pub black: PlayerInfo,
    /// 白プレイヤー．
    pub white: PlayerInfo,
}

/// プレイヤー個別情報．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    /// プレイヤー名 ( `Player::name()` 由来)．
    pub name: String,
    /// 任意のパラメータ ( seed や simulations 数等)．
    pub params: serde_json::Value,
}

impl PlayerInfo {
    /// パラメータなしで名前のみのプレイヤー情報を作る．
    #[must_use]
    pub fn just_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// 終局結果の記録．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameResultRecord {
    /// 勝者 ( 引き分けなら `None`)．
    pub winner: Option<Color>,
    /// 終局時の石数．
    pub score: Score,
}

/// 終局時の石数．
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Score {
    /// 黒石数．
    pub black: u32,
    /// 白石数．
    pub white: u32,
}

/// 1 着手分のエントリ．
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveEntry {
    /// 手数 ( 1 起点)．
    pub n: u32,
    /// 着手側．
    pub side: Color,
    /// 着手 ( `Move::Place` または `Move::Pass`)．
    #[serde(rename = "move")]
    pub r#move: Move,
    /// 着手時刻．
    pub ts: DateTime<FixedOffset>,
}
