//! `OthelloMultiEnv` の統合テスト．

use othello_core::Color;
use othello_rl::prelude::*;

#[test]
fn alternating_actions_to_terminal() {
    let mut env = OthelloMultiEnv::new(MultiEnvConfig::default());
    env.reset(Some(0));
    let mut prev_agent: Option<String> = None;
    let mut alternations = 0;
    let mut safety = 200;
    while !env.state().is_terminal() && safety > 0 {
        let cur = env.current_agent();
        if let Some(prev) = &prev_agent
            && prev != &cur
        {
            alternations += 1;
        }
        prev_agent = Some(cur.clone());
        let mask = env.action_mask(&cur);
        let idx = mask
            .iter()
            .enumerate()
            .find(|(_, b)| **b)
            .map(|(i, _)| i)
            .unwrap_or(64);
        env.step(Action(idx as u32)).unwrap();
        safety -= 1;
    }
    assert!(env.state().is_terminal());
    assert!(alternations > 5, "agents should alternate");
}

#[test]
fn final_rewards_zero_sum() {
    let mut env = OthelloMultiEnv::new(MultiEnvConfig::default());
    env.reset(Some(42));
    let mut safety = 200;
    while !env.state().is_terminal() && safety > 0 {
        let cur = env.current_agent();
        let mask = env.action_mask(&cur);
        let idx = mask
            .iter()
            .enumerate()
            .find(|(_, b)| **b)
            .map(|(i, _)| i)
            .unwrap_or(64);
        env.step(Action(idx as u32)).unwrap();
        safety -= 1;
    }
    let rb = env.rewards()["black"];
    let rw = env.rewards()["white"];
    assert!((rb + rw).abs() < 1e-6, "sparse rewards should be zero-sum");
}

#[test]
fn observe_shape_correct_for_each_agent() {
    let env = OthelloMultiEnv::new(MultiEnvConfig::default());
    let ob = env.observe("black");
    let ow = env.observe("white");
    assert_eq!(ob.shape(), vec![3, 8, 8]);
    assert_eq!(ow.shape(), vec![3, 8, 8]);
}

#[test]
fn illegal_action_errors() {
    let mut env = OthelloMultiEnv::new(MultiEnvConfig::default());
    env.reset(Some(0));
    // 黒手番で中央 ( idx 36) は石があり置けない
    let r = env.step(Action(36));
    assert!(r.is_err());
}

#[test]
fn out_of_range_errors() {
    let mut env = OthelloMultiEnv::new(MultiEnvConfig::default());
    env.reset(Some(0));
    let r = env.step(Action(999));
    assert!(matches!(r, Err(RlError::OutOfRange { .. })));
}

#[test]
fn current_agent_initially_black() {
    let env = OthelloMultiEnv::new(MultiEnvConfig::default());
    assert_eq!(env.current_agent(), "black");
    assert_eq!(env.state().side_to_move, Color::Black);
}
