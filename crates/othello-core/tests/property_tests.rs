//! プロパティベーステスト ( proptest)．
//!
//! 検証する性質:
//! 1. ランダム合法手を選び続ければ，最大 200 手以内で必ず終局に達する
//! 2. 8×8 で `Bitboard8` と `GenericBoard` に同じ手列を適用したとき，
//!    全ステップで石配置・合法手集合・反転座標集合が一致する ( 同値性)
//! 3. 終局時には連続パス 2 回 か 盤面満杯 のいずれかが成立する
//! 4. apply の戻り値 ( 反転座標) は必ず 1 つ以上 ( Place 時)
//! 5. 反転座標は全て相手色から自色に変わっている

use othello_core::generic_board::GenericBoard;
use othello_core::prelude::*;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;

/// ランダム自己対戦を行い，最終ゲーム状態と総手数を返す．
///
/// 最大 `max_steps` 手で打ち切り．
fn play_random_game_8x8(seed: u64, max_steps: u32) -> (GameState, u32) {
    let mut state = GameState::standard_8x8();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut steps = 0u32;
    while !state.is_terminal() && steps < max_steps {
        let moves = state.legal_moves();
        if moves.is_empty() {
            state.apply_move(Move::Pass).unwrap();
        } else {
            let mv = *moves.choose(&mut rng).unwrap();
            state.apply_move(mv).unwrap();
        }
        steps += 1;
    }
    (state, steps)
}

proptest! {
    /// 性質 1: ランダム合法手を続けると 200 手以内で必ず終局する．
    ///
    /// 性質 3: 終局時には以下のいずれかが成立する．
    ///   (a) 連続パス 2 回 ( consecutive_passes >= 2)
    ///   (b) 盤面満杯 ( empty_count == 0)
    ///   (c) 両者とも合法手なし ( has_any_legal_move(black) == false かつ
    ///       has_any_legal_move(white) == false) — 空マスはあるが両者置けない局面
    ///
    /// (c) は (a) と区別される: 終局判定で先に board.is_terminal() が真になった場合，
    /// 連続パス回数は 0 のまま終局する．
    #[test]
    fn random_play_terminates_within_200_steps(seed in 0u64..10_000) {
        let (state, steps) = play_random_game_8x8(seed, 200);
        prop_assert!(state.is_terminal(), "game did not terminate within 200 steps (got {} steps)", steps);
        let no_legal_either = !state.board.has_any_legal_move(Color::Black)
            && !state.board.has_any_legal_move(Color::White);
        prop_assert!(
            state.consecutive_passes >= 2 || state.board.empty_count() == 0 || no_legal_either,
            "terminal condition not met: cps={}, empty={}, no_legal_either={}",
            state.consecutive_passes,
            state.board.empty_count(),
            no_legal_either
        );
    }

    /// 性質 2: 8×8 における Bitboard8 と GenericBoard の同値性．
    ///
    /// 同じシードで同じ手列を作り，両実装で並行に進めて全ステップで一致を確認する．
    #[test]
    fn bitboard_and_generic_equivalence_8x8(seed in 0u64..5_000) {
        let mut bb_state = GameState::standard_8x8();
        let mut gen_board = GenericBoard::standard(BoardSize::STANDARD).unwrap();
        let mut gen_side = Color::Black;
        let mut gen_consec_pass = 0u8;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        for _step in 0..200 {
            // 終局判定の一致
            let bb_terminal = bb_state.is_terminal();
            let gen_terminal = gen_board.is_terminal()
                || gen_consec_pass >= 2;
            prop_assert_eq!(bb_terminal, gen_terminal, "terminal mismatch");
            if bb_terminal {
                break;
            }

            // 合法手集合の一致 ( BTreeSet で順序非依存比較)
            let bb_moves: BTreeSet<Coord> = bb_state
                .legal_moves()
                .iter()
                .filter_map(|m| m.coord())
                .collect();
            let gen_moves: BTreeSet<Coord> = gen_board
                .legal_moves(gen_side)
                .iter()
                .filter_map(|m| m.coord())
                .collect();
            prop_assert_eq!(&bb_moves, &gen_moves, "legal moves mismatch");

            // 着手選択 ( 同じ rng 状態なので両者で同じ手が選ばれる)
            let mv = if bb_moves.is_empty() {
                Move::Pass
            } else {
                bb_state
                    .legal_moves()
                    .choose(&mut rng)
                    .copied()
                    .unwrap()
            };

            // 両者に同じ手を適用
            let bb_flips = bb_state.apply_move(mv).unwrap();
            let gen_flips = gen_board.apply(gen_side, mv).unwrap();

            // 反転座標集合の一致
            let bb_flip_set: BTreeSet<Coord> = bb_flips.iter().copied().collect();
            let gen_flip_set: BTreeSet<Coord> = gen_flips.iter().copied().collect();
            prop_assert_eq!(bb_flip_set, gen_flip_set, "flip coord mismatch");

            // GenericBoard 側の手番・連続パス更新を Bitboard8 側に揃える
            match mv {
                Move::Pass => gen_consec_pass = gen_consec_pass.saturating_add(1),
                Move::Place(_) => gen_consec_pass = 0,
            }
            gen_side = gen_side.opponent();

            // 全マス比較
            for r in 0..8u8 {
                for c in 0..8u8 {
                    let coord = Coord::new(r, c);
                    prop_assert_eq!(
                        bb_state.board.cell(coord),
                        gen_board.cell(coord),
                        "cell mismatch at ({},{})", r, c
                    );
                }
            }
        }
    }

    /// 性質 4 & 5: 着手成功時，反転座標は 1 つ以上で，全て自色になっている．
    #[test]
    fn flips_are_nonempty_and_become_own(seed in 0u64..3_000) {
        let mut state = GameState::standard_8x8();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        for _ in 0..200 {
            if state.is_terminal() { break; }
            let moves = state.legal_moves();
            if moves.is_empty() {
                state.apply_move(Move::Pass).unwrap();
                continue;
            }
            let mv = *moves.choose(&mut rng).unwrap();
            let side = state.side_to_move;
            let flips = state.apply_move(mv).unwrap();
            prop_assert!(!flips.is_empty(), "Place move must flip at least one stone");
            for f in &flips {
                // apply 後は自色 ( = 着手前の手番) になっているはず
                prop_assert_eq!(
                    state.board.cell(*f),
                    Some(side),
                    "flipped stone at ({},{}) should be {:?}", f.row, f.col, side
                );
            }
        }
    }

    /// 性質 ( 各種サイズ): 4×4 / 6×6 / 10×10 でランダム自己対戦が終局する．
    #[test]
    fn variable_size_termination(seed in 0u64..500, side_len in prop::sample::select(vec![4u8, 6, 10])) {
        let size = BoardSize::square(side_len);
        let mut board = GenericBoard::standard(size).unwrap();
        let mut side = Color::Black;
        let mut consec_pass = 0u8;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let max_steps = (side_len as u32) * (side_len as u32) * 2 + 10;
        let mut steps = 0u32;
        loop {
            let terminal = consec_pass >= 2 || board.is_terminal();
            if terminal || steps >= max_steps { break; }
            let moves = board.legal_moves(side);
            if moves.is_empty() {
                board.apply(side, Move::Pass).unwrap();
                consec_pass += 1;
            } else {
                let mv = *moves.choose(&mut rng).unwrap();
                board.apply(side, mv).unwrap();
                consec_pass = 0;
            }
            side = side.opponent();
            steps += 1;
        }
        let terminated = consec_pass >= 2 || board.is_terminal();
        prop_assert!(terminated, "size {} did not terminate after {} steps", side_len, steps);
    }
}
