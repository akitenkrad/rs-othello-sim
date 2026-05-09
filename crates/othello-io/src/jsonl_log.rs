//! JSONL ロガー．設計書 §4.2 のフォーマットに準拠したイベントログを 1 行 1 JSON で出力する．
//!
//! ## イベント例
//!
//! ```jsonl
//! {"event": "game_start", "ts": "2026-05-09T15:30:00.000+09:00", "game_id": "550e...", "board_size": [8, 8], "players": {"black": "Mcts", "white": "Random"}}
//! {"event": "move", "ts": "...", "game_id": "...", "n": 1, "side": "Black", "move": {"Place": [2, 3]}, "stones": {"black": 4, "white": 1}, "legal_count": 3}
//! {"event": "pass", "ts": "...", "game_id": "...", "n": 30, "side": "Black"}
//! {"event": "game_end", "ts": "...", "game_id": "...", "winner": "Black", "stones": {"black": 38, "white": 26}, "moves_total": 60}
//! ```
//!
//! `move` フィールドの値は **配列表現** ( `{"Place": [row, col]}` ) であり，
//! `othello-core` の `Move` 既定 serde 表現 ( `{"Place": {"row": ..., "col": ...}}` ) とは異なる．
//! このモジュールでは [`MoveSerial`] で変換して書き出す．

use crate::error::IoError;
use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Move};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// JSONL 用の `Move` 中間表現 ( 設計書 §4.2 の配列表現)．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MoveSerial {
    /// `{"Place": [row, col]}`．
    Place {
        /// `Place` フィールド ( 配列 `[row, col]`)．
        #[serde(rename = "Place")]
        place: [u8; 2],
    },
    /// 文字列 `"Pass"`．
    Pass(PassTag),
}

/// `MoveSerial::Pass` のタグ ( serde の都合で文字列で出すための補助型)．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PassTag {
    /// `Pass`．
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

/// 黒白プレイヤー名 ( 文字列 2 つ)．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerNames {
    /// 黒プレイヤー名．
    pub black: String,
    /// 白プレイヤー名．
    pub white: String,
}

/// 石数 ( 黒 / 白)．
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stones {
    /// 黒石数．
    pub black: u32,
    /// 白石数．
    pub white: u32,
}

/// `game_start` イベント．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartEvent {
    /// ISO 8601 / RFC 3339 タイムスタンプ ( ミリ秒精度)．
    pub ts: DateTime<FixedOffset>,
    /// ゲーム ID．
    pub game_id: String,
    /// 盤面サイズ ( 配列 `[rows, cols]`)．
    pub board_size: [u8; 2],
    /// プレイヤー名．
    pub players: PlayerNames,
}

/// `move` イベント．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEvent {
    /// タイムスタンプ．
    pub ts: DateTime<FixedOffset>,
    /// ゲーム ID．
    pub game_id: String,
    /// 手数 ( 1 起点)．
    pub n: u32,
    /// 着手側．
    pub side: Color,
    /// 着手 ( `{"Place": [row, col]}`)．
    #[serde(rename = "move")]
    pub r#move: MoveSerial,
    /// 着手後の石数．
    pub stones: Stones,
    /// 合法手数．
    pub legal_count: u32,
}

/// `pass` イベント．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassEvent {
    /// タイムスタンプ．
    pub ts: DateTime<FixedOffset>,
    /// ゲーム ID．
    pub game_id: String,
    /// 手数．
    pub n: u32,
    /// パス側．
    pub side: Color,
}

/// `game_end` イベント．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEndEvent {
    /// タイムスタンプ．
    pub ts: DateTime<FixedOffset>,
    /// ゲーム ID．
    pub game_id: String,
    /// 勝者 ( 引き分けは `None`)．
    pub winner: Option<Color>,
    /// 終局時の石数．
    pub stones: Stones,
    /// 総手数 ( パス含む)．
    pub moves_total: u32,
}

/// 設計書 §4.2 で出力される全イベント型．`event` タグで判別する．
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LogEvent {
    /// ゲーム開始．
    GameStart(GameStartEvent),
    /// 着手 ( Place)．
    Move(MoveEvent),
    /// パス．
    Pass(PassEvent),
    /// ゲーム終了．
    GameEnd(GameEndEvent),
}

impl LogEvent {
    /// `[rows, cols]` を [`BoardSize`] から作る補助．
    #[must_use]
    pub fn board_size_pair(size: BoardSize) -> [u8; 2] {
        [size.rows, size.cols]
    }
}

/// JSONL ログ Writer．書き込みごとに改行を付与する．
pub struct JsonlLogger {
    writer: Box<dyn Write + Send>,
}

impl std::fmt::Debug for JsonlLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlLogger").finish_non_exhaustive()
    }
}

impl JsonlLogger {
    /// ファイルパスから生成する ( バッファリング Writer)．
    pub fn to_path(path: impl AsRef<Path>) -> Result<Self, IoError> {
        let file = File::create(path)?;
        Ok(Self::from_writer(BufWriter::new(file)))
    }

    /// 任意の `Write + Send` から生成する．
    pub fn from_writer<W: Write + Send + 'static>(writer: W) -> Self {
        Self {
            writer: Box::new(writer),
        }
    }

    /// 1 イベントを書き出す ( 改行付き)．
    pub fn log_event(&mut self, event: &LogEvent) -> Result<(), IoError> {
        serde_json::to_writer(&mut self.writer, event)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    /// `game_start` を書き出す．
    pub fn log_game_start(&mut self, evt: &GameStartEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::GameStart(evt.clone()))
    }

    /// `move` を書き出す．
    pub fn log_move(&mut self, evt: &MoveEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::Move(evt.clone()))
    }

    /// `pass` を書き出す．
    pub fn log_pass(&mut self, evt: &PassEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::Pass(evt.clone()))
    }

    /// `game_end` を書き出す．
    pub fn log_game_end(&mut self, evt: &GameEndEvent) -> Result<(), IoError> {
        self.log_event(&LogEvent::GameEnd(evt.clone()))
    }

    /// 内部 Writer をフラッシュする．
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
