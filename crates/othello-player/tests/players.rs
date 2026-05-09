//! 各プレイヤーの動作統合テスト．

use othello_core::prelude::*;
use othello_player::{GreedyPlayer, HumanPlayer, Player, PlayerError, RandomPlayer};
use std::io::BufReader;

#[test]
fn random_player_finishes_game_via_self_play() {
    let mut black = RandomPlayer::with_seed(Color::Black, 1);
    let mut white = RandomPlayer::with_seed(Color::White, 2);
    let mut state = GameState::standard_8x8();
    let mut steps = 0;
    while !state.is_terminal() && steps < 200 {
        let mv = if state.side_to_move == Color::Black {
            black.select_move(&state).unwrap()
        } else {
            white.select_move(&state).unwrap()
        };
        // 合法手なしのときは Pass を強制
        let mv = if state.legal_moves().is_empty() {
            Move::Pass
        } else {
            mv
        };
        state.apply_move(mv).unwrap();
        steps += 1;
    }
    assert!(state.is_terminal(), "Random vs Random should terminate");
}

#[test]
fn greedy_player_always_picks_legal_move() {
    let mut p = GreedyPlayer::with_color(Color::Black);
    let s = GameState::standard_8x8();
    let mv = p.select_move(&s).unwrap();
    assert!(s.legal_moves().contains(&mv));
}

#[test]
fn human_player_handles_multiple_inputs() {
    let input = "d3\npass\ne4\n";
    let output: Vec<u8> = Vec::new();
    let mut p = HumanPlayer::new_with_io(
        "Human",
        Color::Black,
        BufReader::new(input.as_bytes()),
        output,
    );
    let s = GameState::standard_8x8();
    assert_eq!(p.select_move(&s).unwrap(), Move::Place(Coord::new(2, 3)));
    assert_eq!(p.select_move(&s).unwrap(), Move::Pass);
    assert_eq!(p.select_move(&s).unwrap(), Move::Place(Coord::new(3, 4)));
}

#[test]
fn human_player_returns_error_on_eof() {
    let input = "";
    let output: Vec<u8> = Vec::new();
    let mut p = HumanPlayer::new_with_io(
        "Human",
        Color::Black,
        BufReader::new(input.as_bytes()),
        output,
    );
    let s = GameState::standard_8x8();
    let r = p.select_move(&s);
    assert!(matches!(r, Err(PlayerError::InputExhausted)));
}
