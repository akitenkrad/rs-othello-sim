//! [`OthelloMultiEnv`]: PettingZoo (AECEnv) 互換のマルチエージェント Othello 環境．

use crate::action_space::Action;
use crate::error::RlError;
use crate::observation::{Observation, ObservationType, make_observation};
use crate::reward::{RewardFn, RewardMode, make_reward_fn};
use ndarray::Array1;
use othello_core::{BoardSize, Color, GameState, Move};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// マルチエージェントの ID．`"black"` または `"white"`．
pub type AgentId = String;

/// 環境設定．
#[derive(Debug, Clone)]
pub struct MultiEnvConfig {
    /// 盤面サイズ．
    pub board_size: BoardSize,
    /// 観測形式．
    pub observation_type: ObservationType,
    /// 報酬モード．
    pub reward_mode: RewardMode,
    /// 打ち切り手数．
    pub max_steps: Option<u32>,
}

impl Default for MultiEnvConfig {
    fn default() -> Self {
        Self {
            board_size: BoardSize::STANDARD,
            observation_type: ObservationType::Planes,
            reward_mode: RewardMode::Sparse,
            max_steps: None,
        }
    }
}

/// マルチエージェント環境．
pub struct OthelloMultiEnv {
    state: GameState,
    config: MultiEnvConfig,
    move_history: Vec<Move>,
    rewards: HashMap<AgentId, f32>,
    terminations: HashMap<AgentId, bool>,
    truncations: HashMap<AgentId, bool>,
    step_count: u32,
    #[allow(dead_code)]
    rng: ChaCha8Rng,
}

/// `step` の戻り値．
#[derive(Debug, Clone)]
pub struct MultiStepResult {
    /// 各 agent の観測．
    pub observations: HashMap<AgentId, Observation>,
    /// 各 agent の報酬．
    pub rewards: HashMap<AgentId, f32>,
    /// 各 agent の終局フラグ．
    pub terminations: HashMap<AgentId, bool>,
    /// 各 agent の打ち切りフラグ．
    pub truncations: HashMap<AgentId, bool>,
    /// 補助情報．
    pub info: super::env::StepInfo,
}

impl OthelloMultiEnv {
    /// 設定から環境を作る．
    #[must_use]
    pub fn new(config: MultiEnvConfig) -> Self {
        let state = GameState::standard(config.board_size).expect("valid board size");
        let mut rewards = HashMap::new();
        rewards.insert("black".to_string(), 0.0);
        rewards.insert("white".to_string(), 0.0);
        let mut terminations = HashMap::new();
        terminations.insert("black".to_string(), false);
        terminations.insert("white".to_string(), false);
        let mut truncations = HashMap::new();
        truncations.insert("black".to_string(), false);
        truncations.insert("white".to_string(), false);
        Self {
            state,
            config,
            move_history: Vec::new(),
            rewards,
            terminations,
            truncations,
            step_count: 0,
            rng: ChaCha8Rng::seed_from_u64(0),
        }
    }

    /// 環境をリセットし全 agent の初期観測を返す．
    pub fn reset(&mut self, seed: Option<u64>) -> HashMap<AgentId, Observation> {
        let s = seed.unwrap_or(0);
        self.rng = ChaCha8Rng::seed_from_u64(s);
        self.state = GameState::standard(self.config.board_size).expect("valid board size");
        self.move_history.clear();
        self.step_count = 0;
        for v in self.rewards.values_mut() {
            *v = 0.0;
        }
        for v in self.terminations.values_mut() {
            *v = false;
        }
        for v in self.truncations.values_mut() {
            *v = false;
        }
        self.observations()
    }

    /// 全 agent ID．
    #[must_use]
    pub fn agents(&self) -> Vec<AgentId> {
        vec!["black".to_string(), "white".to_string()]
    }

    /// 現在の手番 agent．
    #[must_use]
    pub fn current_agent(&self) -> AgentId {
        match self.state.side_to_move {
            Color::Black => "black".into(),
            Color::White => "white".into(),
        }
    }

    /// 指定 agent の観測．
    #[must_use]
    pub fn observe(&self, agent: &str) -> Observation {
        let view = agent_to_color(agent);
        make_observation(
            &self.state,
            view,
            self.config.observation_type,
            &self.move_history,
        )
    }

    /// 指定 agent の合法手マスク．
    #[must_use]
    pub fn action_mask(&self, agent: &str) -> Array1<bool> {
        let view = agent_to_color(agent);
        let size = self.config.board_size;
        let len = Action::space_size(size) as usize;
        let mut mask = Array1::from_elem((len,), false);
        if self.state.side_to_move != view {
            return mask;
        }
        let legal = self.state.legal_moves();
        if legal.is_empty() {
            mask[len - 1] = true;
        } else {
            for mv in legal {
                let a = Action::from_move(mv, size);
                mask[a.0 as usize] = true;
            }
        }
        mask
    }

    /// 1 手適用する．現在の `current_agent` の手として扱う．
    pub fn step(&mut self, action: Action) -> Result<MultiStepResult, RlError> {
        action.validate(self.config.board_size)?;
        if self.state.is_terminal() {
            return Ok(MultiStepResult {
                observations: self.observations(),
                rewards: self.rewards.clone(),
                terminations: self.terminations.clone(),
                truncations: self.truncations.clone(),
                info: self.info(),
            });
        }

        let acting = self.state.side_to_move;
        let acting_id = match acting {
            Color::Black => "black",
            Color::White => "white",
        }
        .to_string();
        let opp_id = match acting {
            Color::Black => "white",
            Color::White => "black",
        }
        .to_string();

        // 合法性チェック
        let legal = self.state.legal_moves();
        let agent_move = action.to_move(self.config.board_size);
        let is_legal = if legal.is_empty() {
            agent_move == Move::Pass
        } else {
            legal.contains(&agent_move)
        };
        if !is_legal {
            return Err(RlError::IllegalAction {
                action: action.0,
                legal_count: legal.len() as u32,
            });
        }

        let prev = self.state.clone();

        self.state.apply_move(agent_move)?;
        self.move_history.push(agent_move);
        self.step_count = self.step_count.saturating_add(1);

        let terminated = self.state.is_terminal();
        let truncated = match self.config.max_steps {
            Some(limit) => !terminated && self.step_count >= limit,
            None => false,
        };

        // 報酬計算
        let r_acting = self.compute_reward(&prev, acting, terminated);
        let r_opp = self.compute_reward(&prev, acting.opponent(), terminated);
        self.rewards.insert(acting_id, r_acting);
        self.rewards.insert(opp_id.clone(), r_opp);

        if terminated || truncated {
            for v in self.terminations.values_mut() {
                *v = terminated;
            }
            for v in self.truncations.values_mut() {
                *v = truncated;
            }
        }

        Ok(MultiStepResult {
            observations: self.observations(),
            rewards: self.rewards.clone(),
            terminations: self.terminations.clone(),
            truncations: self.truncations.clone(),
            info: self.info(),
        })
    }

    /// 各 agent の報酬辞書を借用で返す．
    #[inline]
    #[must_use]
    pub fn rewards(&self) -> &HashMap<AgentId, f32> {
        &self.rewards
    }

    /// 各 agent の終局辞書を借用で返す．
    #[inline]
    #[must_use]
    pub fn terminations(&self) -> &HashMap<AgentId, bool> {
        &self.terminations
    }

    /// 各 agent の打ち切り辞書を借用で返す．
    #[inline]
    #[must_use]
    pub fn truncations(&self) -> &HashMap<AgentId, bool> {
        &self.truncations
    }

    /// 内部状態の参照．
    #[inline]
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    fn observations(&self) -> HashMap<AgentId, Observation> {
        let mut m = HashMap::new();
        for a in self.agents() {
            m.insert(a.clone(), self.observe(&a));
        }
        m
    }

    fn info(&self) -> super::env::StepInfo {
        let cur = self.current_agent();
        let mask = self.action_mask(&cur);
        let legal_count = mask.iter().filter(|b| **b).count() as u32;
        super::env::StepInfo {
            action_mask: mask,
            legal_count,
            move_number: self.state.move_number,
            side_to_move: self.state.side_to_move,
        }
    }

    fn compute_reward(&self, prev: &GameState, view: Color, terminated: bool) -> f32 {
        let f: Box<dyn RewardFn> = make_reward_fn(self.config.reward_mode);
        f.compute(prev, &self.state, view, terminated)
    }
}

fn agent_to_color(agent: &str) -> Color {
    match agent {
        "black" => Color::Black,
        "white" => Color::White,
        other => panic!("unknown agent id: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> OthelloMultiEnv {
        OthelloMultiEnv::new(MultiEnvConfig::default())
    }

    #[test]
    fn reset_returns_observations_for_both() {
        let mut e = env();
        let obs = e.reset(Some(1));
        assert!(obs.contains_key("black"));
        assert!(obs.contains_key("white"));
        assert_eq!(obs["black"].shape(), vec![3, 8, 8]);
    }

    #[test]
    fn current_agent_alternates_by_side() {
        let mut e = env();
        e.reset(Some(0));
        assert_eq!(e.current_agent(), "black");
        // 1 手進めれば white の手番
        let mask = e.action_mask("black");
        let idx = mask
            .iter()
            .enumerate()
            .find(|(_, b)| **b)
            .map(|(i, _)| i)
            .unwrap();
        e.step(Action(idx as u32)).unwrap();
        assert_eq!(e.current_agent(), "white");
    }

    #[test]
    fn action_mask_only_for_acting_agent() {
        let mut e = env();
        e.reset(Some(0));
        let bm = e.action_mask("black");
        let wm = e.action_mask("white");
        assert!(bm.iter().any(|b| *b));
        assert!(wm.iter().all(|b| !*b));
    }

    #[test]
    fn full_rollout_terminates() {
        let mut e = env();
        e.reset(Some(0));
        let mut safety = 200u32;
        while !e.state.is_terminal() && safety > 0 {
            let cur = e.current_agent();
            let mask = e.action_mask(&cur);
            let idx = mask
                .iter()
                .enumerate()
                .find(|(_, b)| **b)
                .map(|(i, _)| i)
                .unwrap_or(64);
            e.step(Action(idx as u32)).unwrap();
            safety -= 1;
        }
        assert!(e.state.is_terminal());
        // 終局後は両 agent の terminations が true
        assert!(e.terminations()["black"]);
        assert!(e.terminations()["white"]);
        // 報酬は zero-sum
        let rb = e.rewards()["black"];
        let rw = e.rewards()["white"];
        assert!((rb + rw).abs() < 1e-6);
    }
}
