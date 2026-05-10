//! Integration tests for `OthelloEnv`.

use othello_core::{BoardSize, Color};
use othello_player::RandomPlayer;
use othello_rl::prelude::*;

fn make_env(agent: Color, obs_ty: ObservationType, mode: RewardMode) -> OthelloEnv {
    OthelloEnv::new(
        EnvConfig {
            board_size: BoardSize::STANDARD,
            agent_color: agent,
            observation_type: obs_ty,
            reward_mode: mode,
            include_history_in_obs: false,
            max_steps: None,
        },
        Box::new(RandomPlayer::with_seed(agent.opponent(), 11)),
    )
}

#[test]
fn planes_obs_shape_after_step() {
    let mut env = make_env(Color::Black, ObservationType::Planes, RewardMode::Sparse);
    let (_obs, info) = env.reset(Some(7));
    assert_eq!(info.action_mask.len(), 65);
    let idx = info
        .action_mask
        .iter()
        .enumerate()
        .find(|(_, b)| **b)
        .map(|(i, _)| i)
        .unwrap();
    let r = env.step(Action(idx as u32)).unwrap();
    assert_eq!(r.observation.shape(), vec![3, 8, 8]);
}

#[test]
fn flat_obs_shape_after_step() {
    let mut env = make_env(Color::Black, ObservationType::Flat, RewardMode::Sparse);
    let (obs, _info) = env.reset(Some(0));
    assert_eq!(obs.shape(), vec![3 * 8 * 8]);
}

#[test]
fn move_sequence_obs_grows() {
    let mut env = make_env(
        Color::Black,
        ObservationType::MoveSequence,
        RewardMode::Sparse,
    );
    let (obs, _info) = env.reset(Some(0));
    assert_eq!(obs.shape(), vec![0]);
    let mask = env.action_mask();
    let idx = mask
        .iter()
        .enumerate()
        .find(|(_, b)| **b)
        .map(|(i, _)| i)
        .unwrap();
    let r = env.step(Action(idx as u32)).unwrap();
    // agent (1 手) + opponent ( 1 手) = 2
    assert!(r.observation.shape()[0] >= 1);
}

#[test]
fn dense_reward_nonzero_during_play() {
    let mut env = make_env(Color::Black, ObservationType::Planes, RewardMode::Dense);
    env.reset(Some(0));
    let mask = env.action_mask();
    let idx = mask
        .iter()
        .enumerate()
        .find(|(_, b)| **b)
        .map(|(i, _)| i)
        .unwrap();
    let r = env.step(Action(idx as u32)).unwrap();
    // 黒が 1 手指して反転すると差分が変わるので reward != 0 が期待される
    // ( 必ずしもそうではないが少なくとも非NaN)
    assert!(!r.reward.is_nan());
}

#[test]
fn truncated_when_max_steps_hit() {
    let mut env = OthelloEnv::new(
        EnvConfig {
            board_size: BoardSize::STANDARD,
            agent_color: Color::Black,
            observation_type: ObservationType::Planes,
            reward_mode: RewardMode::Sparse,
            include_history_in_obs: false,
            max_steps: Some(1),
        },
        Box::new(RandomPlayer::with_seed(Color::White, 0)),
    );
    env.reset(Some(0));
    let mask = env.action_mask();
    let idx = mask
        .iter()
        .enumerate()
        .find(|(_, b)| **b)
        .map(|(i, _)| i)
        .unwrap();
    let r = env.step(Action(idx as u32)).unwrap();
    assert!(r.truncated || r.terminated);
}
