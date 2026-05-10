//! MCTS tree-reuse A/B benchmark (Phase 6.2).
//!
//! Compares `tree_reuse = false` (the baseline) and `tree_reuse = true`
//! over one full game with the same `simulations` count. A single game
//! is played against a random opponent with a fixed seed.
//!
//! The actual benchmark run (`cargo bench`) is not part of CI; the
//! compile check is performed via
//! `cargo bench --no-run -p othello-player`.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use othello_core::prelude::*;
use othello_player::{MctsConfig, MctsPlayer, Player, RandomPlayer};

/// Plays one full MCTS (Black) vs Random (White) game and returns the
/// total number of moves played.
fn play_one_game(black: &mut MctsPlayer, white: &mut RandomPlayer) -> u32 {
    let mut state = GameState::standard_8x8();
    let mut safety = 200u32;
    while !state.is_terminal() && safety > 0 {
        let legal = state.legal_moves();
        let mv = if legal.is_empty() {
            Move::Pass
        } else if state.side_to_move == Color::Black {
            black.select_move(&state).expect("mcts move")
        } else {
            white.select_move(&state).expect("random move")
        };
        if state.apply_move(mv).is_err() {
            break;
        }
        safety -= 1;
    }
    state.move_number
}

fn bench_tree_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("mcts_tree_reuse");
    let sims: u32 = 200;

    group.bench_function("no_reuse_8x8_full_game", |b| {
        b.iter(|| {
            let mut black =
                MctsPlayer::new(Color::Black, MctsConfig::new(sims).with_tree_reuse(false))
                    .with_seed(0xC0FFEE);
            let mut white = RandomPlayer::with_seed(Color::White, 0xBEEF);
            let n = play_one_game(&mut black, &mut white);
            black_box(n)
        });
    });

    group.bench_function("with_reuse_8x8_full_game", |b| {
        b.iter(|| {
            let mut black =
                MctsPlayer::new(Color::Black, MctsConfig::new(sims).with_tree_reuse(true))
                    .with_seed(0xC0FFEE);
            let mut white = RandomPlayer::with_seed(Color::White, 0xBEEF);
            let n = play_one_game(&mut black, &mut white);
            black_box(n)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tree_reuse);
criterion_main!(benches);
