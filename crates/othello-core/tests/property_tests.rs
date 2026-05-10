//! Property-based tests (proptest).
//!
//! Properties under test:
//! 1. Repeatedly choosing random legal moves always terminates within 200
//!    moves.
//! 2. Applying the same move sequence to `Bitboard8` and `GenericBoard`
//!    on an 8x8 board yields matching stone positions, legal-move sets,
//!    and flip-coordinate sets at every step (equivalence).
//! 3. At termination either two consecutive passes have occurred or the
//!    board is full.
//! 4. The return value of `apply` (flipped coordinates) is non-empty for
//!    `Place` moves.
//! 5. Every flipped coordinate has been changed from the opponent's color
//!    to ours.

use othello_core::generic_board::GenericBoard;
use othello_core::prelude::*;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;

/// Plays a random self-play game and returns the final game state and the
/// total number of moves played.
///
/// Aborts after at most `max_steps` moves.
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
    /// Property 1: continually playing random legal moves terminates
    /// within 200 moves.
    ///
    /// Property 3: at termination one of the following holds:
    ///   (a) two consecutive passes (`consecutive_passes >= 2`),
    ///   (b) the board is full (`empty_count == 0`),
    ///   (c) neither side has any legal move
    ///       (`has_any_legal_move(black) == false` and
    ///       `has_any_legal_move(white) == false`) — empty squares exist
    ///       but neither side can play.
    ///
    /// (c) is distinct from (a): if `board.is_terminal()` becomes true
    /// first, the game terminates while `consecutive_passes` is still 0.
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

    /// Property 2: equivalence of `Bitboard8` and `GenericBoard` on the
    /// standard 8x8 board.
    ///
    /// Generates the same move sequence from a shared seed, advances both
    /// implementations in parallel, and verifies that they agree at every
    /// step.
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

    /// Properties 4 & 5: when a move succeeds, the set of flipped
    /// coordinates is non-empty and every flipped stone now matches the
    /// player's color.
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

    /// Property (various sizes): random self-play terminates on 4x4, 6x6,
    /// and 10x10 boards.
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
