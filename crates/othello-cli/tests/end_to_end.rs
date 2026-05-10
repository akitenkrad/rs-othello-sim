//! End-to-end: simulate one Random vs Random game, save it as JSON,
//! reload it, replay it through `Replayer`, and verify that the final
//! score matches.

use othello_core::prelude::*;
use othello_engine::prelude::*;
use othello_io::prelude::*;
use othello_player::RandomPlayer;
use std::io::Cursor;

#[test]
fn random_vs_random_save_load_replay_consistency() {
    // 1) シミュレーション
    let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
    let mut b = RandomPlayer::with_seed(Color::Black, 1);
    let mut w = RandomPlayer::with_seed(Color::White, 2);
    let original_result = engine.run(&mut b, &mut w).unwrap();

    // 2) JSON 保存
    let record = engine.into_record(PlayerPair {
        black: PlayerInfo {
            name: "RandomPlayer".into(),
            params: serde_json::json!({"seed": 1}),
        },
        white: PlayerInfo {
            name: "RandomPlayer".into(),
            params: serde_json::json!({"seed": 2}),
        },
    });
    let mut buf = Vec::new();
    JsonWriter::new().write_game(&mut buf, &record).unwrap();

    // 3) 読込
    let parsed = JsonReader::new().read_game(Cursor::new(&buf)).unwrap();
    assert_eq!(record, parsed);

    // 4) 棋譜から history を再構築 ( replayer 用) → 着手列をすべて適用
    let mut state = GameState::standard_8x8();
    let mut history = GameHistory::new(state.clone());
    for entry in &parsed.moves {
        state.apply_move(entry.r#move).unwrap();
        history.push(entry.r#move, state.clone());
    }

    // 5) Replayer で進めて状態が一致すること
    let mut replayer = Replayer::new(&history);
    for _ in 0..history.total_moves() {
        replayer.step_forward();
    }
    let final_state = replayer.current();
    assert!(final_state.is_terminal());
    let replayed_result = final_state.result().unwrap();
    assert_eq!(replayed_result.black, original_result.black);
    assert_eq!(replayed_result.white, original_result.white);
    assert_eq!(replayed_result.winner, original_result.winner);
}
