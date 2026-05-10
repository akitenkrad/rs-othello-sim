//! MCTS `tree_reuse` の統合テスト ( Phase 6.2)．

use othello_core::prelude::*;
use othello_player::{GreedyPlayer, MctsConfig, MctsPlayer, Player, RandomPlayer};

/// MCTS ( tree_reuse=true) vs Greedy で 8×8 を 1 局完走．不正手を返さず Pass も適切に扱う．
#[test]
fn mcts_with_tree_reuse_plays_full_game() {
    let mut mcts = MctsPlayer::new(Color::Black, MctsConfig::new(100).with_tree_reuse(true))
        .with_seed(0xC0FFEE);
    let mut greedy = GreedyPlayer::with_color(Color::White);
    let mut state = GameState::standard_8x8();
    let mut steps = 0;
    while !state.is_terminal() && steps < 200 {
        let legal = state.legal_moves();
        let mv = if legal.is_empty() {
            Move::Pass
        } else if state.side_to_move == Color::Black {
            let m = mcts.select_move(&state).unwrap();
            assert!(legal.contains(&m), "MctsPlayer returned illegal move {m:?}");
            m
        } else {
            let m = greedy.select_move(&state).unwrap();
            assert!(
                legal.contains(&m),
                "GreedyPlayer returned illegal move {m:?}"
            );
            m
        };
        state.apply_move(mv).unwrap();
        steps += 1;
    }
    assert!(
        state.is_terminal(),
        "game should terminate within 200 plies"
    );
    let result = state.result().expect("terminal state has a result");
    // 結果は黒勝ち / 白勝ち / 引き分けのいずれかであれば OK ( 強さの保証はしない)．
    let _ = result;
}

/// 4×4 ( Generic 盤面) でも tree_reuse=true で正常動作する．
#[test]
fn mcts_tree_reuse_4x4() {
    let s = GameState::standard(BoardSize::square(4)).unwrap();
    let mut mcts =
        MctsPlayer::new(s.side_to_move, MctsConfig::new(50).with_tree_reuse(true)).with_seed(42);
    let mut rnd = RandomPlayer::with_seed(s.side_to_move.opponent(), 1);
    let mut state = s;
    let mut steps = 0;
    let agent_color = mcts.color();
    while !state.is_terminal() && steps < 100 {
        let legal = state.legal_moves();
        let mv = if legal.is_empty() {
            Move::Pass
        } else if state.side_to_move == agent_color {
            let m = mcts.select_move(&state).unwrap();
            assert!(legal.contains(&m));
            m
        } else {
            rnd.select_move(&state).unwrap()
        };
        state.apply_move(mv).unwrap();
        steps += 1;
    }
    assert!(state.is_terminal());
}

/// tree_reuse=on と off で同じ seed なら，1 手目は同一の手を返す ( fresh tree からスタートする
/// 1 手目は両者で挙動が一致するはず)．
#[test]
fn tree_reuse_first_move_matches_baseline() {
    let mut mcts_off =
        MctsPlayer::new(Color::Black, MctsConfig::new(80).with_tree_reuse(false)).with_seed(2026);
    let mut mcts_on =
        MctsPlayer::new(Color::Black, MctsConfig::new(80).with_tree_reuse(true)).with_seed(2026);
    let s = GameState::standard_8x8();
    let m_off = mcts_off.select_move(&s).unwrap();
    let m_on = mcts_on.select_move(&s).unwrap();
    assert_eq!(
        m_off, m_on,
        "first move must match between tree_reuse off and on"
    );
}
