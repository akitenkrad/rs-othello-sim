//! [`OthelloEnv`]: a Gymnasium-compatible single-agent Othello environment.
//!
//! ## Loop specification
//!
//! 1. `reset` builds the initial position. If `agent_color` is `White`, the
//!    opponent (Black) plays one move first before the first observation is
//!    returned.
//! 2. `step(action)`:
//!    - Returns [`RlError::IllegalAction`] / [`RlError::OutOfRange`] for
//!      invalid moves.
//!    - Applies the agent's move.
//!    - Checks for termination; if terminated, computes the reward and
//!      returns.
//!    - Advances the opponent's turn (auto-pass if the opponent has no legal
//!      moves). After the opponent moves, returns to the agent's turn. If
//!      the agent has no legal moves, auto-passes and returns to the
//!      opponent's turn (two consecutive passes terminate the game).
//!    - Returns observation, reward, terminated, and info.

use crate::action_space::Action;
use crate::error::RlError;
use crate::observation::{Observation, ObservationType, make_observation};
use crate::reward::{RewardFn, RewardMode, make_reward_fn};
use ndarray::Array1;
use othello_core::{BoardSize, Color, GameState, Move};
use othello_player::Player;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Environment configuration.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// Board size.
    pub board_size: BoardSize,
    /// Agent color.
    pub agent_color: Color,
    /// Observation format.
    pub observation_type: ObservationType,
    /// Reward mode.
    pub reward_mode: RewardMode,
    /// Whether to include history (unused in Phase 4; `MoveSequence`
    /// observations always read the history).
    pub include_history_in_obs: bool,
    /// Move-count threshold above which `truncated = true` (use `None` for
    /// unlimited).
    pub max_steps: Option<u32>,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            board_size: BoardSize::STANDARD,
            agent_color: Color::Black,
            observation_type: ObservationType::Planes,
            reward_mode: RewardMode::Sparse,
            include_history_in_obs: false,
            max_steps: None,
        }
    }
}

/// Return value of [`OthelloEnv::step`].
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Observation.
    pub observation: Observation,
    /// Reward.
    pub reward: f32,
    /// Termination flag.
    pub terminated: bool,
    /// Truncation flag (set when `max_steps` is exceeded).
    pub truncated: bool,
    /// Auxiliary information.
    pub info: StepInfo,
}

/// Auxiliary information returned by `step`.
#[derive(Debug, Clone)]
pub struct StepInfo {
    /// Legal-action mask (length = `Action::space_size`).
    pub action_mask: Array1<bool>,
    /// Number of legal moves.
    pub legal_count: u32,
    /// Cumulative move number.
    pub move_number: u32,
    /// Current side to move (agent / opponent, or the final side at
    /// termination).
    pub side_to_move: Color,
}

/// Single-agent Othello environment.
pub struct OthelloEnv {
    state: GameState,
    config: EnvConfig,
    opponent: Box<dyn Player>,
    #[allow(dead_code)]
    rng: ChaCha8Rng,
    move_history: Vec<Move>,
    step_count: u32,
    custom_reward: Option<Box<dyn RewardFn>>,
}

impl OthelloEnv {
    /// Creates an environment from a configuration and an opponent.
    #[must_use]
    pub fn new(config: EnvConfig, opponent: Box<dyn Player>) -> Self {
        let state = GameState::standard(config.board_size).expect("valid board size");
        Self {
            state,
            config,
            opponent,
            rng: ChaCha8Rng::seed_from_u64(0),
            move_history: Vec::new(),
            step_count: 0,
            custom_reward: None,
        }
    }

    /// Sets a custom reward function (builder).
    #[must_use]
    pub fn with_custom_reward(mut self, reward: Box<dyn RewardFn>) -> Self {
        self.custom_reward = Some(reward);
        self
    }

    /// Resets the environment.
    pub fn reset(&mut self, seed: Option<u64>) -> (Observation, StepInfo) {
        let s = seed.unwrap_or(0);
        self.rng = ChaCha8Rng::seed_from_u64(s);
        self.opponent.reset();
        self.state = GameState::standard(self.config.board_size).expect("valid board size");
        self.move_history.clear();
        self.step_count = 0;

        // agent_color が White なら opponent ( Black) が 1 手先に指す．
        if self.state.side_to_move != self.config.agent_color {
            self.advance_until_agent_or_terminal()
                .expect("opponent first move");
        }

        let obs = self.observation();
        let info = self.info();
        (obs, info)
    }

    /// Applies a single action.
    pub fn step(&mut self, action: Action) -> Result<StepResult, RlError> {
        action.validate(self.config.board_size)?;
        // 終局済みでの step は終局報酬 0 + terminated を返す ( SB3 互換的振る舞い)．
        if self.state.is_terminal() {
            return Ok(StepResult {
                observation: self.observation(),
                reward: 0.0,
                terminated: true,
                truncated: false,
                info: self.info(),
            });
        }

        // agent_color の手番でないと invalid．通常 reset / advance により agent 手番に揃う．
        if self.state.side_to_move != self.config.agent_color {
            return Err(RlError::NotYourTurn {
                side: self.state.side_to_move,
                agent: self.config.agent_color,
            });
        }

        // legal check
        let legal = self.state.legal_moves();
        let legal_count = legal.len() as u32;
        let agent_move = action.to_move(self.config.board_size);
        let is_legal = if legal.is_empty() {
            // パスしか取れない局面．Pass のみ受け付ける．
            agent_move == Move::Pass
        } else {
            legal.contains(&agent_move)
        };
        if !is_legal {
            return Err(RlError::IllegalAction {
                action: action.0,
                legal_count,
            });
        }

        let prev = self.state.clone();

        // agent 手を適用
        self.state.apply_move(agent_move)?;
        self.move_history.push(agent_move);
        self.step_count = self.step_count.saturating_add(1);

        // opponent ターンを進める ( agent 手番に戻るまで)．
        if !self.state.is_terminal() {
            self.advance_until_agent_or_terminal()?;
        }

        let terminated = self.state.is_terminal();
        let truncated = match self.config.max_steps {
            Some(limit) => !terminated && self.step_count >= limit,
            None => false,
        };

        let reward = self.reward(&prev, terminated);

        Ok(StepResult {
            observation: self.observation(),
            reward,
            terminated,
            truncated,
            info: self.info(),
        })
    }

    /// Returns the legal actions for the current position.
    #[must_use]
    pub fn legal_actions(&self) -> Vec<Action> {
        if self.state.side_to_move != self.config.agent_color {
            return Vec::new();
        }
        let legal = self.state.legal_moves();
        if legal.is_empty() {
            vec![Action::pass(self.config.board_size)]
        } else {
            legal
                .iter()
                .map(|m| Action::from_move(*m, self.config.board_size))
                .collect()
        }
    }

    /// Returns the legal-move mask for the current position (length =
    /// `N*M+1`).
    #[must_use]
    pub fn action_mask(&self) -> Array1<bool> {
        let size = self.config.board_size;
        let len = Action::space_size(size) as usize;
        let mut mask = Array1::from_elem((len,), false);
        if self.state.side_to_move != self.config.agent_color {
            return mask;
        }
        let legal = self.state.legal_moves();
        if legal.is_empty() {
            mask[len - 1] = true; // Pass
        } else {
            for mv in legal {
                let a = Action::from_move(mv, size);
                mask[a.0 as usize] = true;
            }
        }
        mask
    }

    /// Renders the board as ASCII.
    #[must_use]
    pub fn render(&self) -> String {
        let size = self.state.board.size();
        let mut s = String::new();
        s.push_str("   ");
        for c in 0..size.cols {
            s.push((b'a' + c) as char);
            s.push(' ');
        }
        s.push('\n');
        for r in 0..size.rows {
            s.push_str(&format!("{:>2} ", r + 1));
            for c in 0..size.cols {
                let ch = match self.state.board.cell(othello_core::Coord::new(r, c)) {
                    Some(Color::Black) => 'X',
                    Some(Color::White) => 'O',
                    None => '.',
                };
                s.push(ch);
                s.push(' ');
            }
            s.push('\n');
        }
        s.push_str(&format!(
            "Move: {}  Side: {:?}\n",
            self.state.move_number, self.state.side_to_move
        ));
        s
    }

    /// Returns a reference to the internal state (for debugging).
    #[inline]
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Returns a reference to the configuration.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &EnvConfig {
        &self.config
    }

    fn observation(&self) -> Observation {
        make_observation(
            &self.state,
            self.config.agent_color,
            self.config.observation_type,
            &self.move_history,
        )
    }

    fn info(&self) -> StepInfo {
        let legal_count = if self.state.side_to_move == self.config.agent_color {
            self.state.legal_moves().len() as u32
        } else {
            0
        };
        StepInfo {
            action_mask: self.action_mask(),
            legal_count,
            move_number: self.state.move_number,
            side_to_move: self.state.side_to_move,
        }
    }

    fn reward(&self, prev: &GameState, terminated: bool) -> f32 {
        let agent = self.config.agent_color;
        if let Some(custom) = &self.custom_reward {
            custom.compute(prev, &self.state, agent, terminated)
        } else {
            let f = make_reward_fn(self.config.reward_mode);
            f.compute(prev, &self.state, agent, terminated)
        }
    }

    /// Advances the opponent's turn until control returns to the agent.
    /// Auto-passes if the agent has no legal moves (two consecutive passes
    /// terminate the game).
    fn advance_until_agent_or_terminal(&mut self) -> Result<(), RlError> {
        let agent = self.config.agent_color;
        loop {
            if self.state.is_terminal() {
                break;
            }
            if self.state.side_to_move == agent {
                // agent 手番だが合法手なし → 自動 Pass
                if self.state.legal_moves().is_empty() {
                    self.state.apply_move(Move::Pass)?;
                    self.move_history.push(Move::Pass);
                    continue;
                }
                break;
            }
            // opponent ターン
            let legal = self.state.legal_moves();
            let mv = if legal.is_empty() {
                Move::Pass
            } else {
                self.opponent.select_move(&self.state)?
            };
            self.state.apply_move(mv)?;
            self.move_history.push(mv);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_player::RandomPlayer;

    fn random_env() -> OthelloEnv {
        OthelloEnv::new(
            EnvConfig {
                board_size: BoardSize::STANDARD,
                agent_color: Color::Black,
                observation_type: ObservationType::Planes,
                reward_mode: RewardMode::Sparse,
                include_history_in_obs: false,
                max_steps: None,
            },
            Box::new(RandomPlayer::with_seed(Color::White, 42)),
        )
    }

    #[test]
    fn reset_returns_initial_observation() {
        let mut env = random_env();
        let (obs, info) = env.reset(Some(1));
        assert_eq!(obs.shape(), vec![3, 8, 8]);
        // 初期局面: 黒の手番なので legal_count = 4
        assert_eq!(info.legal_count, 4);
        assert_eq!(info.side_to_move, Color::Black);
    }

    #[test]
    fn step_progresses_state() {
        let mut env = random_env();
        env.reset(Some(2));
        let mask = env.action_mask();
        // mask の true index を 1 つ取って step
        let idx = mask
            .iter()
            .enumerate()
            .find(|(_, b)| **b)
            .map(|(i, _)| i)
            .unwrap();
        let r = env.step(Action(idx as u32)).unwrap();
        assert!(r.info.move_number >= 1);
        assert!(!r.terminated);
    }

    #[test]
    fn illegal_action_returns_error() {
        let mut env = random_env();
        env.reset(Some(3));
        // 中央 ( 4*8+4 = 36) は石があり置けない ( 黒の合法手は外周)．
        let r = env.step(Action(36));
        assert!(matches!(r, Err(RlError::IllegalAction { .. })));
    }

    #[test]
    fn out_of_range_action_returns_error() {
        let mut env = random_env();
        env.reset(Some(4));
        let r = env.step(Action(1000));
        assert!(matches!(r, Err(RlError::OutOfRange { .. })));
    }

    #[test]
    fn agent_white_lets_opponent_move_first() {
        let mut env = OthelloEnv::new(
            EnvConfig {
                board_size: BoardSize::STANDARD,
                agent_color: Color::White,
                observation_type: ObservationType::Planes,
                reward_mode: RewardMode::Sparse,
                include_history_in_obs: false,
                max_steps: None,
            },
            Box::new(RandomPlayer::with_seed(Color::Black, 7)),
        );
        let (_, info) = env.reset(Some(0));
        // 1 手進んでいるので move_number == 1，side_to_move == White
        assert_eq!(info.move_number, 1);
        assert_eq!(info.side_to_move, Color::White);
    }

    #[test]
    fn action_mask_length_correct() {
        let env = random_env();
        let mask = env.action_mask();
        assert_eq!(mask.len(), 65);
    }

    #[test]
    fn rollout_to_termination() {
        let mut env = random_env();
        env.reset(Some(11));
        let mut safety = 200u32;
        loop {
            let mask = env.action_mask();
            if mask.iter().all(|b| !b) {
                // agent 手番でなくなった ( 終局済み)
                break;
            }
            let idx = mask
                .iter()
                .enumerate()
                .find(|(_, b)| **b)
                .map(|(i, _)| i)
                .unwrap();
            let r = env.step(Action(idx as u32)).unwrap();
            if r.terminated {
                let reward = r.reward;
                assert!(reward == -1.0 || reward == 0.0 || reward == 1.0);
                break;
            }
            safety -= 1;
            assert!(safety > 0, "should terminate within 200 steps");
        }
    }
}
