//! JSON ラウンドトリップのプロパティテスト ( 任意 GameRecord)．

use chrono::{DateTime, FixedOffset};
use othello_core::{BoardSize, Color, Coord, Move};
use othello_io::prelude::*;
use proptest::prelude::*;

fn arb_color() -> impl Strategy<Value = Color> {
    prop_oneof![Just(Color::Black), Just(Color::White)]
}

fn arb_move() -> impl Strategy<Value = Move> {
    prop_oneof![
        (0u8..8, 0u8..8).prop_map(|(r, c)| Move::Place(Coord::new(r, c))),
        Just(Move::Pass),
    ]
}

fn arb_ts() -> impl Strategy<Value = DateTime<FixedOffset>> {
    Just(DateTime::parse_from_rfc3339("2026-05-09T15:30:00.000+09:00").unwrap())
}

fn arb_move_entry(n: u32) -> impl Strategy<Value = MoveEntry> {
    (arb_color(), arb_move(), arb_ts()).prop_map(move |(side, mv, ts)| MoveEntry {
        n,
        side,
        r#move: mv,
        ts,
    })
}

fn arb_record() -> impl Strategy<Value = GameRecord> {
    (1usize..30, arb_ts()).prop_flat_map(|(len, started)| {
        let entries: Vec<_> = (1..=len).map(|i| arb_move_entry(i as u32)).collect();
        entries.prop_map(move |moves| {
            GameRecord::new(
                GameMetadata {
                    id: "11111111-2222-3333-4444-555555555555".into(),
                    started_at: started,
                    ended_at: Some(started),
                    board_size: BoardSize::STANDARD,
                    players: PlayerPair {
                        black: PlayerInfo::just_name("X"),
                        white: PlayerInfo::just_name("Y"),
                    },
                    result: Some(GameResultRecord {
                        winner: Some(Color::Black),
                        score: Score {
                            black: 32,
                            white: 32,
                        },
                    }),
                    engine_version: "test".into(),
                },
                moves,
            )
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn json_round_trip_idempotent(record in arb_record()) {
        let mut buf = Vec::new();
        JsonWriter::new().write_game(&mut buf, &record).unwrap();
        let parsed = JsonReader::new().read_game(buf.as_slice()).unwrap();
        prop_assert_eq!(record, parsed);
    }
}
