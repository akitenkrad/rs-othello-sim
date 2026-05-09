//! 8×8 合法手生成のベンチマーク．
//!
//! 設計書 §9 の目標値: $\geq 10^7$ 回/秒 ( Bitboard)．
//!
//! ランダムに進めた局面 1000 個を事前生成し，各局面について
//! 黒/白それぞれの合法手を全列挙するスループットを測る．

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use othello_core::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

fn random_positions(n: usize, seed: u64) -> Vec<Bitboard8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let mut state = GameState::standard_8x8();
        // ランダムに 0..=40 手進める ( 序中盤局面を主に集める)
        let target_moves: u32 = rand::Rng::gen_range(&mut rng, 0..=40_u32);
        let mut consec_pass = 0u8;
        for _ in 0..target_moves {
            if state.is_terminal() {
                break;
            }
            let moves = state.legal_moves();
            if moves.is_empty() {
                state.apply_move(Move::Pass).unwrap();
                consec_pass += 1;
                if consec_pass >= 2 {
                    break;
                }
                continue;
            }
            consec_pass = 0;
            let chosen = *moves.choose(&mut rng).unwrap();
            let _ = state.apply_move(chosen);
        }
        if let Board::Bitboard8(bb) = state.board {
            out.push(bb);
        }
    }
    out
}

fn bench_legal_moves(c: &mut Criterion) {
    let positions = random_positions(1000, 42);
    let mut group = c.benchmark_group("bitboard8_legal_moves");
    group.throughput(Throughput::Elements(positions.len() as u64));

    group.bench_function("legal_mask_black_white", |b| {
        b.iter(|| {
            let mut acc: u64 = 0;
            for pos in &positions {
                acc ^= pos.legal_mask(Color::Black);
                acc ^= pos.legal_mask(Color::White);
            }
            black_box(acc)
        });
    });

    group.bench_function("legal_moves_vec_black", |b| {
        b.iter(|| {
            let mut count = 0usize;
            for pos in &positions {
                count += pos.legal_moves(Color::Black).len();
            }
            black_box(count)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_legal_moves);
criterion_main!(benches);
