//! JSON / GGF のラウンドトリップテスト．

use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Coord, Move};
use othello_io::prelude::*;

fn ts(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

fn fixture() -> GameRecord {
    GameRecord::new(
        GameMetadata {
            id: "550e8400-e29b-41d4-a716-446655440000".into(),
            started_at: ts("2026-05-09T15:30:00.000+09:00"),
            ended_at: Some(ts("2026-05-09T15:30:42.123+09:00")),
            board_size: BoardSize::STANDARD,
            players: PlayerPair {
                black: PlayerInfo {
                    name: "RandomPlayer".into(),
                    params: serde_json::json!({"seed": 42}),
                },
                white: PlayerInfo {
                    name: "GreedyPlayer".into(),
                    params: serde_json::json!({}),
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
            MoveEntry {
                n: 3,
                side: Color::Black,
                r#move: Move::Pass,
                ts: ts("2026-05-09T15:30:00.789+09:00"),
            },
        ],
    )
}

#[test]
fn json_round_trip_preserves_record() {
    let record = fixture();
    let mut buf = Vec::new();
    JsonWriter::new().write_game(&mut buf, &record).unwrap();
    let parsed = JsonReader::new().read_game(buf.as_slice()).unwrap();
    assert_eq!(record, parsed);
}

#[test]
fn ggf_round_trip_preserves_moves() {
    let record = fixture();
    let mut buf = Vec::new();
    GgfWriter::new().write_game(&mut buf, &record).unwrap();
    let parsed = GgfReader::new().read_game(buf.as_slice()).unwrap();
    let original: Vec<_> = record.moves.iter().map(|m| (m.side, m.r#move)).collect();
    let parsed_moves: Vec<_> = parsed.moves.iter().map(|m| (m.side, m.r#move)).collect();
    assert_eq!(original, parsed_moves);
    assert_eq!(parsed.metadata.players.black.name, "RandomPlayer");
    assert_eq!(parsed.metadata.players.white.name, "GreedyPlayer");
}

#[test]
fn ggf_thinking_time_is_accepted() {
    let ggf = "(;GM[Othello]PB[A]PW[B]B[D3//1.234]W[C5//0.001]B[--];)";
    let parsed = GgfReader::new().read_game(ggf.as_bytes()).unwrap();
    assert_eq!(parsed.moves.len(), 3);
    assert_eq!(parsed.moves[0].r#move, Move::Place(Coord::new(2, 3)));
    assert_eq!(parsed.moves[1].r#move, Move::Place(Coord::new(4, 2)));
    assert_eq!(parsed.moves[2].r#move, Move::Pass);
}
