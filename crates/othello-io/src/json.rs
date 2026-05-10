//! Reader/Writer for the native JSON game-record format. Uses
//! `serde_json` against the schema defined in §4.1 of the design
//! document.

use crate::error::IoError;
use crate::record::GameRecord;
use crate::traits::{GameRecordReader, GameRecordWriter};
use std::io::{Read, Write};

/// JSON game-record reader.
#[derive(Debug, Default)]
pub struct JsonReader;

impl JsonReader {
    /// Constructs a new reader.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl GameRecordReader for JsonReader {
    fn read_game<R: Read>(&mut self, source: R) -> Result<GameRecord, IoError> {
        let record: GameRecord = serde_json::from_reader(source)?;
        if record.schema_version != crate::record::SCHEMA_VERSION {
            // 互換性のため警告のみ ( 1.x の minor バージョン違いを許容したい場合は緩和可能)
            // ここではメジャーバージョンが違ったら拒否する単純な実装に留める
            let major = record.schema_version.split('.').next().unwrap_or_default();
            let expected_major = crate::record::SCHEMA_VERSION
                .split('.')
                .next()
                .unwrap_or_default();
            if major != expected_major {
                return Err(IoError::UnsupportedSchemaVersion(record.schema_version));
            }
        }
        Ok(record)
    }
}

/// JSON game-record writer (pretty-printed output).
#[derive(Debug, Default)]
pub struct JsonWriter;

impl JsonWriter {
    /// Constructs a new writer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl GameRecordWriter for JsonWriter {
    fn write_game<W: Write>(&mut self, dest: W, record: &GameRecord) -> Result<(), IoError> {
        serde_json::to_writer_pretty(dest, record)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{
        GameMetadata, GameResultRecord, MoveEntry, PlayerInfo, PlayerPair, SCHEMA_VERSION, Score,
    };
    use chrono::{DateTime, FixedOffset};
    use othello_core::{BoardSize, Color, Coord, Move};

    fn ts(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn sample_record() -> GameRecord {
        GameRecord::new(
            GameMetadata {
                id: "550e8400-e29b-41d4-a716-446655440000".into(),
                started_at: ts("2026-05-09T15:30:00.000+09:00"),
                ended_at: Some(ts("2026-05-09T15:30:42.123+09:00")),
                board_size: BoardSize::STANDARD,
                players: PlayerPair {
                    black: PlayerInfo {
                        name: "MctsPlayer".into(),
                        params: serde_json::json!({"simulations": 1000}),
                    },
                    white: PlayerInfo {
                        name: "RandomPlayer".into(),
                        params: serde_json::json!({"seed": 42}),
                    },
                },
                result: Some(GameResultRecord {
                    winner: Some(Color::Black),
                    score: Score {
                        black: 38,
                        white: 26,
                    },
                }),
                engine_version: "rs-othello-sim 0.1.0".into(),
            },
            vec![
                MoveEntry {
                    n: 1,
                    side: Color::Black,
                    r#move: Move::Place(Coord::new(2, 3)),
                    ts: ts("2026-05-09T15:30:00.123+09:00"),
                },
                MoveEntry {
                    n: 2,
                    side: Color::White,
                    r#move: Move::Place(Coord::new(2, 2)),
                    ts: ts("2026-05-09T15:30:00.456+09:00"),
                },
            ],
        )
    }

    #[test]
    fn round_trip() {
        let record = sample_record();
        let mut buf = Vec::new();
        JsonWriter::new().write_game(&mut buf, &record).unwrap();
        let parsed = JsonReader::new().read_game(buf.as_slice()).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn schema_fields_match_spec() {
        let record = sample_record();
        let mut buf = Vec::new();
        JsonWriter::new().write_game(&mut buf, &record).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // 必須キー
        assert!(s.contains(&format!("\"schema_version\": \"{SCHEMA_VERSION}\"")));
        assert!(s.contains("\"metadata\""));
        assert!(s.contains("\"moves\""));
        // metadata 内
        assert!(s.contains("\"id\""));
        assert!(s.contains("\"started_at\""));
        assert!(s.contains("\"ended_at\""));
        assert!(s.contains("\"board_size\""));
        assert!(s.contains("\"players\""));
        assert!(s.contains("\"result\""));
        assert!(s.contains("\"engine_version\""));
        // move エントリ
        assert!(s.contains("\"n\": 1"));
        assert!(s.contains("\"side\": \"Black\""));
        assert!(s.contains("\"move\""));
        // Move serde 形式
        assert!(s.contains("\"Place\""));
    }

    #[test]
    fn pass_serializes_correctly() {
        let mut record = sample_record();
        record.moves.push(MoveEntry {
            n: 3,
            side: Color::Black,
            r#move: Move::Pass,
            ts: ts("2026-05-09T15:30:00.789+09:00"),
        });
        let mut buf = Vec::new();
        JsonWriter::new().write_game(&mut buf, &record).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"Pass\""));
    }

    #[test]
    fn rejects_unknown_major_schema() {
        let json = r#"{"schema_version":"2.0","metadata":{"id":"x","started_at":"2026-05-09T15:30:00.000+09:00","ended_at":null,"board_size":{"rows":8,"cols":8},"players":{"black":{"name":"A","params":{}},"white":{"name":"B","params":{}}},"result":null,"engine_version":"v"},"moves":[]}"#;
        let r = JsonReader::new().read_game(json.as_bytes());
        assert!(matches!(r, Err(IoError::UnsupportedSchemaVersion(_))));
    }
}
