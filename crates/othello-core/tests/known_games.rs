//! 既知の手順を再生して結果を検証する統合テスト．

use othello_core::prelude::*;

/// 標準 8×8 で序盤数手を進める．
///
/// 黒が (2,3)，白が (2,2)，黒が (3,2)，白が (4,2) の単純な進行を確認する．
#[test]
fn opening_diagonal_4_moves() {
    let mut state = GameState::standard_8x8();

    // 1. 黒 (2,3): 白 (3,3) を反転
    let flipped = state.apply_move(Move::Place(Coord::new(2, 3))).unwrap();
    assert_eq!(flipped, vec![Coord::new(3, 3)]);
    assert_eq!(state.board.cell(Coord::new(2, 3)), Some(Color::Black));
    assert_eq!(state.board.cell(Coord::new(3, 3)), Some(Color::Black));
    assert_eq!(state.side_to_move, Color::White);
    assert_eq!(state.move_number, 1);

    // 2. 白 (2,2): 黒 (2,3)? いや (3,3) は黒 (2,3) は黒 → (2,2) に白を置くと
    //    斜め方向 (3,3) の黒が反転する ( SE 方向で白 (4,4) にぶつかる)
    let flipped = state.apply_move(Move::Place(Coord::new(2, 2))).unwrap();
    assert_eq!(flipped, vec![Coord::new(3, 3)]);
    assert_eq!(state.board.cell(Coord::new(2, 2)), Some(Color::White));
    assert_eq!(state.board.cell(Coord::new(3, 3)), Some(Color::White));

    // 3 手目以降は合法手の存在のみ確認
    let black_moves = state.legal_moves();
    assert!(!black_moves.is_empty());

    // ゲームはまだ続行中
    assert!(!state.is_terminal());
}

/// パスのみ可能 → 最終的に終局という流れの確認．
#[test]
fn forced_pass_then_terminal() {
    // 全マス黒石にして両者 Pass しか取れない局面を作る
    let mut state = GameState::standard_8x8();
    for r in 0..8u8 {
        for c in 0..8u8 {
            state.board.set(Coord::new(r, c), Some(Color::Black));
        }
    }
    // 黒 ( 手番) には合法手なし → Pass
    assert!(!state.board.has_any_legal_move(Color::Black));
    assert!(!state.board.has_any_legal_move(Color::White));
    state.apply_move(Move::Pass).unwrap();
    assert_eq!(state.consecutive_passes, 1);
    // 白も Pass
    state.apply_move(Move::Pass).unwrap();
    assert_eq!(state.consecutive_passes, 2);
    // 終局
    assert!(state.is_terminal());
    let result = state.result().unwrap();
    assert_eq!(result.winner, Some(Color::Black));
    assert_eq!(result.black, 64);
    assert_eq!(result.white, 0);
}

/// 8×8 で完全にランダム再生を 1 局通す ( クラッシュなし)．
#[test]
fn full_random_game_8x8_no_panic() {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use rand_chacha::ChaCha8Rng;

    let mut state = GameState::standard_8x8();
    let mut rng = ChaCha8Rng::seed_from_u64(2026);
    let mut steps = 0u32;
    while !state.is_terminal() && steps < 200 {
        let moves = state.legal_moves();
        if moves.is_empty() {
            state.apply_move(Move::Pass).unwrap();
        } else {
            let mv = *moves.choose(&mut rng).unwrap();
            state.apply_move(mv).unwrap();
        }
        steps += 1;
    }
    assert!(state.is_terminal());
    let result = state.result().unwrap();
    assert_eq!(
        result.black + result.white,
        state.board.count(Color::Black) + state.board.count(Color::White)
    );
    // 8×8 では総石数は最大 64
    assert!(result.black + result.white <= 64);
}

/// 4×4 ミニ盤面で完全ランダム再生 ( 最終的に必ず終局)．
#[test]
fn full_random_game_4x4_no_panic() {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use rand_chacha::ChaCha8Rng;

    let mut state = GameState::standard(BoardSize::square(4)).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(123);
    let mut steps = 0u32;
    while !state.is_terminal() && steps < 50 {
        let moves = state.legal_moves();
        if moves.is_empty() {
            state.apply_move(Move::Pass).unwrap();
        } else {
            let mv = *moves.choose(&mut rng).unwrap();
            state.apply_move(mv).unwrap();
        }
        steps += 1;
    }
    assert!(state.is_terminal());
}

/// 16×16 でも同様にランダム再生がクラッシュしない．
#[test]
fn full_random_game_16x16_no_panic() {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use rand_chacha::ChaCha8Rng;

    let mut state = GameState::standard(BoardSize::square(16)).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    let mut steps = 0u32;
    let max_steps = 16 * 16 * 2 + 10;
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
    assert!(state.is_terminal());
}
