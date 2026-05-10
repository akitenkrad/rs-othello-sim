//! Benchmarks for random self-play.
//!
//! Targets from design doc §9:
//! - 8x8 random self-play: at least $10^6$ moves per second (single
//!   thread).
//! - 16x16 random self-play: at least $10^4$ moves per second.
//!
//! Throughput is measured with one fully terminated game as a single
//! unit.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use othello_core::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

fn play_random_game(rng: &mut ChaCha8Rng, size: BoardSize) -> u32 {
    let mut state = GameState::standard(size).unwrap();
    while !state.is_terminal() {
        let moves = state.legal_moves();
        if moves.is_empty() {
            state.apply_move(Move::Pass).unwrap();
        } else {
            let mv = *moves.choose(rng).unwrap();
            state.apply_move(mv).unwrap();
        }
    }
    state.move_number
}

fn bench_self_play_8x8(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_play_8x8");
    // 1 ベンチ反復で 100 ゲームを回す
    let games_per_iter: u64 = 100;
    group.throughput(Throughput::Elements(games_per_iter));
    group.bench_function("random_vs_random", |b| {
        let mut rng = ChaCha8Rng::seed_from_u64(2026);
        b.iter(|| {
            let mut total = 0u32;
            for _ in 0..games_per_iter {
                total = total.wrapping_add(play_random_game(&mut rng, BoardSize::STANDARD));
            }
            black_box(total)
        });
    });
    group.finish();
}

fn bench_self_play_16x16(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_play_16x16");
    let games_per_iter: u64 = 10;
    group.throughput(Throughput::Elements(games_per_iter));
    group.bench_function("random_vs_random", |b| {
        let mut rng = ChaCha8Rng::seed_from_u64(2026);
        b.iter(|| {
            let mut total = 0u32;
            for _ in 0..games_per_iter {
                total = total.wrapping_add(play_random_game(&mut rng, BoardSize::square(16)));
            }
            black_box(total)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_self_play_8x8, bench_self_play_16x16);
criterion_main!(benches);
