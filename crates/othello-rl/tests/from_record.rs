//! Behavioural tests for `transitions_from_record` /
//! `transitions_from_record_both_sides`.

use chrono::TimeZone;
use othello_core::{BoardSize, Color, Coord, GameState, Move};
use othello_io::{
    GameMetadata, GameRecord, GameResultRecord, MoveEntry, PlayerInfo, PlayerPair, SCHEMA_VERSION,
    Score,
};
use othello_rl::replay_buffer::{transitions_from_record, transitions_from_record_both_sides};

fn ts() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::FixedOffset::east_opt(0)
        .unwrap()
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap()
}

fn metadata(size: BoardSize, winner: Option<Color>, black: u32, white: u32) -> GameMetadata {
    GameMetadata {
        id: "test-game".to_string(),
        started_at: ts(),
        ended_at: Some(ts()),
        board_size: size,
        players: PlayerPair {
            black: PlayerInfo::just_name("black"),
            white: PlayerInfo::just_name("white"),
        },
        result: Some(GameResultRecord {
            winner,
            score: Score { black, white },
        }),
        engine_version: "test".to_string(),
    }
}

fn move_entry(n: u32, side: Color, mv: Move) -> MoveEntry {
    MoveEntry {
        n,
        side,
        r#move: mv,
        ts: ts(),
    }
}

/// Builds a simple 8x8 game record where Black wins.
fn record_black_wins() -> GameRecord {
    let size = BoardSize::STANDARD;
    // 黒の典型的な合法手 ( d3 = (2, 3))
    let moves = vec![
        move_entry(1, Color::Black, Move::Place(Coord::new(2, 3))),
        move_entry(2, Color::White, Move::Place(Coord::new(2, 2))),
        move_entry(3, Color::Black, Move::Place(Coord::new(2, 1))),
    ];
    GameRecord {
        schema_version: SCHEMA_VERSION.to_string(),
        metadata: metadata(size, Some(Color::Black), 5, 1),
        moves,
    }
}

/// Game record that (by construction) ends in a draw.
fn record_draw() -> GameRecord {
    let size = BoardSize::STANDARD;
    let moves = vec![
        move_entry(1, Color::Black, Move::Place(Coord::new(2, 3))),
        move_entry(2, Color::White, Move::Place(Coord::new(2, 2))),
    ];
    GameRecord {
        schema_version: SCHEMA_VERSION.to_string(),
        metadata: metadata(size, None, 32, 32),
        moves,
    }
}

#[test]
fn black_view_returns_one_transition_per_black_move() {
    let rec = record_black_wins();
    let ts = transitions_from_record(&rec, Color::Black).unwrap();
    // 黒手は 2 手 ( 1 番目と 3 番目)
    assert_eq!(ts.len(), 2);
    for t in &ts {
        assert_eq!(t.side, Color::Black);
        // 自分視点で勝者なので value = +1
        assert!((t.value - 1.0).abs() < 1e-6);
        assert_eq!(t.observation.shape(), &[3, 8, 8]);
        assert_eq!(t.legal_mask.len(), 65);
    }
}

#[test]
fn white_view_returns_one_transition_per_white_move() {
    let rec = record_black_wins();
    let ts = transitions_from_record(&rec, Color::White).unwrap();
    assert_eq!(ts.len(), 1);
    let t = &ts[0];
    assert_eq!(t.side, Color::White);
    // 白は負け
    assert!((t.value - (-1.0)).abs() < 1e-6);
}

#[test]
fn both_sides_concat_black_then_white() {
    let rec = record_black_wins();
    let all = transitions_from_record_both_sides(&rec).unwrap();
    // 黒 2 手 + 白 1 手
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].side, Color::Black);
    assert_eq!(all[1].side, Color::Black);
    assert_eq!(all[2].side, Color::White);
}

#[test]
fn draw_yields_zero_value_for_both_sides() {
    let rec = record_draw();
    let all = transitions_from_record_both_sides(&rec).unwrap();
    for t in &all {
        assert!(
            t.value.abs() < 1e-6,
            "value should be 0 for draw, got {}",
            t.value
        );
    }
}

#[test]
fn observation_obeys_view_color() {
    // 1 手目，黒の手番．黒視点では own=2，opp=2 (planes [0,1] それぞれ 2 セル)．
    let rec = record_black_wins();
    let ts = transitions_from_record(&rec, Color::Black).unwrap();
    let obs = &ts[0].observation;
    let own_sum: f32 = obs.slice(ndarray::s![0, .., ..]).sum();
    let opp_sum: f32 = obs.slice(ndarray::s![1, .., ..]).sum();
    // 初期局面: 黒石 2 個 / 白石 2 個
    assert!((own_sum - 2.0).abs() < 1e-6);
    assert!((opp_sum - 2.0).abs() < 1e-6);
    // legal mask は黒の手番なので 4 セル立っている ( + Pass の最後の bit は false)
    let mask_true_count = ts[0].legal_mask.iter().filter(|b| **b).count();
    assert_eq!(mask_true_count, 4);
}

#[test]
fn invalid_record_side_mismatch_rejected() {
    // 1 手目を白の手番として記録する ( 不正)
    let mut rec = record_black_wins();
    rec.moves[0].side = Color::White;
    let r = transitions_from_record(&rec, Color::Black);
    assert!(matches!(r, Err(othello_rl::ReplayError::InvalidRecord(_))));
}

#[test]
fn legal_mask_consistent_with_state() {
    // 任意の長さの一連の手を辿りつつ，各 transition の mask が真値の合法手と
    // 一致することを確認．
    let rec = record_black_wins();
    let ts = transitions_from_record(&rec, Color::Black).unwrap();
    // 手動で再現
    let mut state = GameState::standard_8x8();
    let size = BoardSize::STANDARD;
    let mut idx = 0;
    for entry in &rec.moves {
        if state.side_to_move == Color::Black {
            let truth = state.legal_moves();
            let mut expected = [false; 65];
            if truth.is_empty() {
                expected[64] = true;
            } else {
                for m in truth {
                    let a = othello_rl::Action::from_move(m, size);
                    expected[a.0 as usize] = true;
                }
            }
            for (k, e) in expected.iter().enumerate() {
                assert_eq!(
                    ts[idx].legal_mask[k], *e,
                    "mask differs at idx {idx}, cell {k}"
                );
            }
            idx += 1;
        }
        state.apply_move(entry.r#move).unwrap();
    }
}
