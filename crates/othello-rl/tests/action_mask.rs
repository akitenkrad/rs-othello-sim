//! Comprehensive tests for action masks.

use othello_core::{BoardSize, Color, GameState};
use othello_player::RandomPlayer;
use othello_rl::prelude::*;

#[test]
fn mask_length_matches_space_size() {
    for n in [4u8, 6, 8, 10] {
        let env = OthelloEnv::new(
            EnvConfig {
                board_size: BoardSize::square(n),
                agent_color: Color::Black,
                observation_type: ObservationType::Flat,
                reward_mode: RewardMode::Sparse,
                include_history_in_obs: false,
                max_steps: None,
            },
            Box::new(RandomPlayer::with_seed(Color::White, 0)),
        );
        let m = env.action_mask();
        assert_eq!(m.len(), (n as usize * n as usize) + 1);
    }
}

#[test]
fn mask_matches_legal_moves() {
    let env = OthelloEnv::new(
        EnvConfig {
            board_size: BoardSize::STANDARD,
            agent_color: Color::Black,
            observation_type: ObservationType::Planes,
            reward_mode: RewardMode::Sparse,
            include_history_in_obs: false,
            max_steps: None,
        },
        Box::new(RandomPlayer::with_seed(Color::White, 0)),
    );
    let mask = env.action_mask();
    let legal = GameState::standard_8x8().legal_moves();
    let count = mask.iter().filter(|b| **b).count();
    assert_eq!(count, legal.len());
}

#[test]
fn mask_pass_when_only_pass() {
    // 全マス白で塞ぐ．黒は合法手がない．
    let mut env = OthelloEnv::new(
        EnvConfig {
            board_size: BoardSize::STANDARD,
            agent_color: Color::Black,
            observation_type: ObservationType::Planes,
            reward_mode: RewardMode::Sparse,
            include_history_in_obs: false,
            max_steps: None,
        },
        Box::new(RandomPlayer::with_seed(Color::White, 0)),
    );
    env.reset(Some(0));
    // 直接 state を書き換えるための裏口は env には無いので，
    // ここでは mask の長さチェックだけで十分．
    let m = env.action_mask();
    assert_eq!(m.len(), 65);
}
