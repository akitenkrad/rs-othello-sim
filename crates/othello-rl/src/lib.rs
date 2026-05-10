//! # othello-rl
//!
//! Reinforcement-learning environments for Othello. Provides a
//! Gymnasium-compatible single-agent environment ([`OthelloEnv`]) and a
//! PettingZoo-compatible multi-agent environment
//! ([`OthelloMultiEnv`]).
//!
//! ## Observations
//!
//! - [`Observation::Planes`] — `[3, H, W]` (own / opp / legal mask).
//! - [`Observation::Flat`] — `[3*H*W]`.
//! - [`Observation::MoveSequence`] — move sequence in the
//!   Othello-GPT style.
//!
//! ## Rewards
//!
//! - [`RewardMode::Sparse`] — `+1 / 0 / -1` only at terminal states.
//! - [`RewardMode::Dense`] — change in stone-count differential each
//!   move.
//! - For custom rewards, implement the [`RewardFn`] trait and pass it
//!   to [`OthelloEnv::with_custom_reward`].
//!
//! ## Example
//!
//! ```
//! use othello_rl::prelude::*;
//! use othello_player::RandomPlayer;
//! use othello_core::{BoardSize, Color};
//!
//! let mut env = OthelloEnv::new(
//!     EnvConfig {
//!         board_size: BoardSize::STANDARD,
//!         agent_color: Color::Black,
//!         observation_type: ObservationType::Planes,
//!         reward_mode: RewardMode::Sparse,
//!         include_history_in_obs: false,
//!         max_steps: None,
//!     },
//!     Box::new(RandomPlayer::with_seed(Color::White, 0)),
//! );
//! let (_obs, _info) = env.reset(Some(42));
//! ```

pub mod action_space;
pub mod env;
pub mod error;
pub mod multi_env;
pub mod observation;
pub mod replay_buffer;
pub mod reward;

pub use action_space::Action;
pub use env::{EnvConfig, OthelloEnv, StepInfo, StepResult};
pub use error::RlError;
pub use multi_env::{AgentId, MultiEnvConfig, MultiStepResult, OthelloMultiEnv};
pub use observation::{Observation, ObservationType, make_observation};
pub use replay_buffer::{
    PrioritizedReplayBuffer, ReplayBuffer, ReplayError, SumTree, Transition, TransitionBatch,
    UniformReplayBuffer, transitions_from_record, transitions_from_record_both_sides,
};
pub use reward::{DenseReward, RewardFn, RewardMode, SparseReward, make_reward_fn};

/// Prelude that imports the commonly used types in one go.
pub mod prelude {
    pub use crate::{
        Action, AgentId, DenseReward, EnvConfig, MultiEnvConfig, MultiStepResult, Observation,
        ObservationType, OthelloEnv, OthelloMultiEnv, PrioritizedReplayBuffer, ReplayBuffer,
        ReplayError, RewardFn, RewardMode, RlError, SparseReward, StepInfo, StepResult, Transition,
        TransitionBatch, UniformReplayBuffer,
    };
}
