//! Integration tests for `GameEngine`.

use othello_core::prelude::*;
use othello_engine::prelude::*;
use othello_io::{PlayerInfo, PlayerPair};
use othello_player::{GreedyPlayer, RandomPlayer};

#[test]
fn random_vs_random_does_not_panic_for_many_seeds() {
    for seed in 0..30u64 {
        let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
        let mut b = RandomPlayer::with_seed(Color::Black, seed);
        let mut w = RandomPlayer::with_seed(Color::White, seed.wrapping_mul(13) ^ 0xdead);
        let r = engine.run(&mut b, &mut w).unwrap();
        // 終局時の合計石数は最大盤面マス数以下
        assert!(r.black + r.white <= 64);
    }
}

#[test]
fn replayer_step_forward_full_then_backward_to_initial() {
    let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
    let mut b = RandomPlayer::with_seed(Color::Black, 11);
    let mut w = GreedyPlayer::with_color(Color::White);
    engine.run(&mut b, &mut w).unwrap();
    let history = engine.history().clone();
    let mut r = Replayer::new(&history);
    let total = history.total_moves();
    for _ in 0..total {
        r.step_forward();
    }
    assert_eq!(r.cursor(), total);
    for _ in 0..total {
        r.step_backward();
    }
    assert_eq!(r.cursor(), 0);
    assert_eq!(r.current(), history.initial());
}

#[test]
fn into_record_serializes_and_replays() {
    let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
    let mut b = RandomPlayer::with_seed(Color::Black, 1);
    let mut w = RandomPlayer::with_seed(Color::White, 2);
    let result = engine.run(&mut b, &mut w).unwrap();
    let record = engine.into_record(PlayerPair {
        black: PlayerInfo::just_name("RandomPlayer"),
        white: PlayerInfo::just_name("RandomPlayer"),
    });

    // moves 列を再生して終局スコアが一致することを確認
    let mut state = GameState::standard_8x8();
    for entry in &record.moves {
        state.apply_move(entry.r#move).unwrap();
    }
    assert!(state.is_terminal());
    let r2 = state.result().unwrap();
    assert_eq!(r2.black, result.black);
    assert_eq!(r2.white, result.white);
    assert_eq!(r2.winner, result.winner);
}
