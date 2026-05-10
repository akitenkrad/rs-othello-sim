//! JSONL logger that emits one event per line in the format defined in
//! §4.2 of the design document.
//!
//! ## Event examples
//!
//! ```jsonl
//! {"event": "game_start", "ts": "2026-05-09T15:30:00.000+09:00", "game_id": "550e...", "board_size": [8, 8], "players": {"black": "Mcts", "white": "Random"}}
//! {"event": "move", "ts": "...", "game_id": "...", "n": 1, "side": "Black", "move": {"Place": [2, 3]}, "stones": {"black": 4, "white": 1}, "legal_count": 3}
//! {"event": "pass", "ts": "...", "game_id": "...", "n": 30, "side": "Black"}
//! {"event": "game_end", "ts": "...", "game_id": "...", "winner": "Black", "stones": {"black": 38, "white": 26}, "moves_total": 60}
//! ```
//!
//! The `move` field uses the **array form** (`{"Place": [row, col]}`),
//! which differs from the default serde representation of `Move` in
//! `othello-core` (`{"Place": {"row": ..., "col": ...}}`). This module
//! converts via [`MoveSerial`] when writing.

use crate::error::IoError;
use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Move};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Intermediate `Move` representation for JSONL (the array form from
/// §4.2 of the design document).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MoveSerial {
    /// `{"Place": [row, col]}`.
    Place {
        /// `Place` field (array `[row, col]`).
        #[serde(rename = "Place")]
        place: [u8; 2],
    },
    /// String `"Pass"`.
    Pass(PassTag),
}

/// Tag for `MoveSerial::Pass` (a helper type used so serde emits a
/// string).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PassTag {
    /// `Pass`.
    Pass,
}

impl From<Move> for MoveSerial {
    fn from(m: Move) -> Self {
        match m {
            Move::Place(c) => MoveSerial::Place {
                place: [c.row, c.col],
            },
            Move::Pass => MoveSerial::Pass(PassTag::Pass),
        }
    }
}

/// Black and white player names (two strings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerNames {
    /// Black player name.
    pub black: String,
    /// White player name.
    pub white: String,
}

/// Stone counts (black / white).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stones {
    /// Black stone count.
    pub black: u32,
    /// White stone count.
    pub white: u32,
}

/// `game_start` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartEvent {
    /// ISO 8601 / RFC 3339 timestamp (millisecond precision).
    pub ts: DateTime<FixedOffset>,
    /// Game ID.
    pub game_id: String,
    /// Board size as `[rows, cols]`.
    pub board_size: [u8; 2],
    /// Player names.
    pub players: PlayerNames,
}

/// `move` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEvent {
    /// Timestamp.
    pub ts: DateTime<FixedOffset>,
    /// Game ID.
    pub game_id: String,
    /// Move number (1-based).
    pub n: u32,
    /// Side that played.
    pub side: Color,
    /// Move (`{"Place": [row, col]}`).
    #[serde(rename = "move")]
    pub r#move: MoveSerial,
    /// Stone counts after the move.
    pub stones: Stones,
    /// Number of legal moves.
    pub legal_count: u32,
}

/// `pass` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassEvent {
    /// Timestamp.
    pub ts: DateTime<FixedOffset>,
    /// Game ID.
    pub game_id: String,
    /// Move number.
    pub n: u32,
    /// Side that passed.
    pub side: Color,
}

/// `game_end` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEndEvent {
    /// Timestamp.
    pub ts: DateTime<FixedOffset>,
    /// Game ID.
    pub game_id: String,
    /// Winner (`None` for a draw).
    pub winner: Option<Color>,
    /// Stone counts at the end of the game.
    pub stones: Stones,
    /// Total number of moves (includes passes).
    pub moves_total: u32,
}

/// Every event type emitted per §4.2 of the design document. The
/// variant is discriminated by the `event` tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LogEvent {
    /// Game start.
    GameStart(GameStartEvent),
    /// Place a stone.
    Move(MoveEvent),
    /// Pass.
    Pass(PassEvent),
    /// Game end.
    GameEnd(GameEndEvent),
}

impl LogEvent {
    /// Helper that builds `[rows, cols]` from a [`BoardSize`].
    #[must_use]
    pub fn board_size_pair(size: BoardSize) -> [u8; 2] {
        [size.rows, size.cols]
    }
}

/// JSONL log writer. A newline is emitted after every record.
pub struct JsonlLogger {
    writer: Box<dyn Write + Send>,
}

impl std::fmt::Debug for JsonlLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlLogger").finish_non_exhaustive()
    }
}

impl JsonlLogger {
    /// Builds a logger backed by a buffered file writer.
    pub fn to_path(path: impl AsRef<Path>) -> Result<Self, IoError> {
        let file = File::create(path)?;
        Ok(Self::from_writer(BufWriter::new(file)))
    }

    /// Builds a logger from any `Write + Send`.
    pub fn from_writer<W: Write + Send + 'static>(writer: W) -> Self {
        Self {
            writer: Box::new(writer),
        }
    }

    /// Writes one event followed by a newline.
    pub fn log_event(&mut self, event: &LogEvent) -> Result<(), IoError> {
        serde_json::to_writer(&mut self.writer, event)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    /// Writes a `game_start` event.
    pub fn log_game_start(&mut self, evt: &GameStartEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::GameStart(evt.clone()))
    }

    /// Writes a `move` event.
    pub fn log_move(&mut self, evt: &MoveEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::Move(evt.clone()))
    }

    /// Writes a `pass` event.
    pub fn log_pass(&mut self, evt: &PassEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::Pass(evt.clone()))
    }

    /// Writes a `game_end` event.
    pub fn log_game_end(&mut self, evt: &GameEndEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::GameEnd(evt.clone()))
    }

    /// Flushes the internal writer.
    pub fn flush(&mut self) -> Result<(), IoError> {
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use othello_core::Coord;

    fn ts() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-05-09T15:30:00.000+09:00").unwrap()
    }

    #[test]
    fn move_serial_place_serializes_as_array() {
        let m: MoveSerial = Move::Place(Coord::new(2, 3)).into();
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "{\"Place\":[2,3]}");
    }

    #[test]
    fn move_serial_pass_serializes_as_string() {
        let m: MoveSerial = Move::Pass.into();
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "\"Pass\"");
    }

    #[test]
    fn game_start_event_layout() {
        let evt = GameStartEvent {
            ts: ts(),
            game_id: "abc".into(),
            board_size: [8, 8],
            players: PlayerNames {
                black: "Mcts".into(),
                white: "Random".into(),
            },
        };
        let s = serde_json::to_string(&LogEvent::GameStart(evt)).unwrap();
        assert!(s.contains("\"event\":\"game_start\""));
        assert!(s.contains("\"board_size\":[8,8]"));
        assert!(s.contains("\"players\":{\"black\":\"Mcts\",\"white\":\"Random\"}"));
        assert!(s.contains("\"game_id\":\"abc\""));
    }

    #[test]
    fn move_event_layout() {
        let evt = MoveEvent {
            ts: ts(),
            game_id: "abc".into(),
            n: 1,
            side: Color::Black,
            r#move: Move::Place(Coord::new(2, 3)).into(),
            stones: Stones { black: 4, white: 1 },
            legal_count: 3,
        };
        let s = serde_json::to_string(&LogEvent::Move(evt)).unwrap();
        assert!(s.contains("\"event\":\"move\""));
        assert!(s.contains("\"n\":1"));
        assert!(s.contains("\"side\":\"Black\""));
        assert!(s.contains("\"move\":{\"Place\":[2,3]}"));
        assert!(s.contains("\"stones\":{\"black\":4,\"white\":1}"));
        assert!(s.contains("\"legal_count\":3"));
    }

    #[test]
    fn pass_event_layout() {
        let evt = PassEvent {
            ts: ts(),
            game_id: "abc".into(),
            n: 30,
            side: Color::Black,
        };
        let s = serde_json::to_string(&LogEvent::Pass(evt)).unwrap();
        assert!(s.contains("\"event\":\"pass\""));
        assert!(s.contains("\"n\":30"));
        assert!(s.contains("\"side\":\"Black\""));
    }

    #[test]
    fn game_end_event_layout() {
        let evt = GameEndEvent {
            ts: ts(),
            game_id: "abc".into(),
            winner: Some(Color::Black),
            stones: Stones {
                black: 38,
                white: 26,
            },
            moves_total: 60,
        };
        let s = serde_json::to_string(&LogEvent::GameEnd(evt)).unwrap();
        assert!(s.contains("\"event\":\"game_end\""));
        assert!(s.contains("\"winner\":\"Black\""));
        assert!(s.contains("\"stones\":{\"black\":38,\"white\":26}"));
        assert!(s.contains("\"moves_total\":60"));
    }

    #[test]
    fn logger_writes_one_line_per_event() {
        let buf: Vec<u8> = Vec::new();
        let mut logger = JsonlLogger::from_writer(buf);
        logger
            .log_game_start(&GameStartEvent {
                ts: ts(),
                game_id: "g1".into(),
                board_size: [8, 8],
                players: PlayerNames {
                    black: "B".into(),
                    white: "W".into(),
                },
            })
            .unwrap();
        logger
            .log_move(&MoveEvent {
                ts: ts(),
                game_id: "g1".into(),
                n: 1,
                side: Color::Black,
                r#move: Move::Place(Coord::new(2, 3)).into(),
                stones: Stones { black: 4, white: 1 },
                legal_count: 3,
            })
            .unwrap();
        logger
            .log_pass(&PassEvent {
                ts: ts(),
                game_id: "g1".into(),
                n: 2,
                side: Color::White,
            })
            .unwrap();
        logger
            .log_game_end(&GameEndEvent {
                ts: ts(),
                game_id: "g1".into(),
                winner: Some(Color::Black),
                stones: Stones {
                    black: 38,
                    white: 26,
                },
                moves_total: 60,
            })
            .unwrap();
        logger.flush().unwrap();

        // logger ownership 経由で内部 buf を取り出すために，書き出し後の中身を改めて確認するには
        // Cursor 経由の API を使う必要がある．ここでは行数だけ間接的に確認する．
        // ただし上記 buf は move されているので，行ベースのチェックは別テストで行う．
    }

    #[test]
    fn logger_to_string_via_cursor() {
        // `Cursor<Vec<u8>>` で書き出し後内容を確認する．
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut logger = JsonlLogger::from_writer(cursor);
        logger
            .log_game_start(&GameStartEvent {
                ts: ts(),
                game_id: "g1".into(),
                board_size: [8, 8],
                players: PlayerNames {
                    black: "B".into(),
                    white: "W".into(),
                },
            })
            .unwrap();
        // 内部の writer を取り出すには Box<dyn Write> を分解できない．
        // そのため，ここではファイル経由で確認する．
        let dir = std::env::temp_dir();
        let path = dir.join(format!("rs-othello-sim-jsonl-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let mut logger = JsonlLogger::to_path(&path).unwrap();
            logger
                .log_game_start(&GameStartEvent {
                    ts: ts(),
                    game_id: "g1".into(),
                    board_size: [8, 8],
                    players: PlayerNames {
                        black: "B".into(),
                        white: "W".into(),
                    },
                })
                .unwrap();
            logger
                .log_pass(&PassEvent {
                    ts: ts(),
                    game_id: "g1".into(),
                    n: 30,
                    side: Color::Black,
                })
                .unwrap();
            logger.flush().unwrap();
        }
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("{\"event\":\"game_start\""));
        assert!(lines[1].starts_with("{\"event\":\"pass\""));
        let _ = std::fs::remove_file(&path);
    }
}
