//! Reward design. Supports `Sparse`, `Dense`, and custom reward functions.

use othello_core::{Color, GameState};
use serde::{Deserialize, Serialize};

/// Built-in reward modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardMode {
    /// Returns +1 (win) / 0 (draw) / -1 (loss) only at terminal states.
    Sparse,
    /// Returns the change in stone-count differential (delta of `own - opp`)
    /// each move. A simplified variant that does not add the final
    /// differential at termination.
    Dense,
}

/// Trait for custom reward functions.
pub trait RewardFn: Send {
    /// Receives a single transition from `prev` to `next` together with the
    /// termination flag from the `agent_color` perspective and returns a
    /// scalar reward for that transition.
    fn compute(
        &self,
        prev: &GameState,
        next: &GameState,
        agent_color: Color,
        terminated: bool,
    ) -> f32;
}

/// Implementation of `RewardMode::Sparse`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SparseReward;

impl RewardFn for SparseReward {
    fn compute(
        &self,
        _prev: &GameState,
        next: &GameState,
        agent_color: Color,
        terminated: bool,
    ) -> f32 {
        if !terminated {
            return 0.0;
        }
        let own = next.board.count(agent_color);
        let opp = next.board.count(agent_color.opponent());
        match own.cmp(&opp) {
            std::cmp::Ordering::Greater => 1.0,
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
        }
    }
}

/// Implementation of `RewardMode::Dense`. Returns the delta of the stone-
/// count differential.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenseReward;

impl RewardFn for DenseReward {
    fn compute(
        &self,
        prev: &GameState,
        next: &GameState,
        agent_color: Color,
        terminated: bool,
    ) -> f32 {
        let opp = agent_color.opponent();
        let prev_diff = prev.board.count(agent_color) as i32 - prev.board.count(opp) as i32;
        let next_diff = next.board.count(agent_color) as i32 - next.board.count(opp) as i32;
        let delta = (next_diff - prev_diff) as f32;
        if terminated {
            // 終局時は勝敗ボーナスを加える
            let bonus = match next_diff.cmp(&0) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Less => -1.0,
                std::cmp::Ordering::Equal => 0.0,
            };
            delta + bonus
        } else {
            delta
        }
    }
}

/// Helper that selects a `RewardFn` from a `RewardMode`.
#[must_use]
pub fn make_reward_fn(mode: RewardMode) -> Box<dyn RewardFn> {
    match mode {
        RewardMode::Sparse => Box::new(SparseReward),
        RewardMode::Dense => Box::new(DenseReward),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::Coord;

    #[test]
    fn sparse_zero_when_not_terminal() {
        let s = GameState::standard_8x8();
        let next = s.clone();
        let r = SparseReward.compute(&s, &next, Color::Black, false);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn sparse_plus_one_on_win() {
        let mut s = GameState::standard_8x8();
        for r in 0..8u8 {
            for c in 0..8u8 {
                s.board.set(Coord::new(r, c), Some(Color::Black));
            }
        }
        let r = SparseReward.compute(&s.clone(), &s, Color::Black, true);
        assert_eq!(r, 1.0);
        let r = SparseReward.compute(&s.clone(), &s, Color::White, true);
        assert_eq!(r, -1.0);
    }

    #[test]
    fn dense_returns_delta() {
        let s = GameState::standard_8x8();
        let mut next = s.clone();
        // 黒石を 1 個追加 ( agent_color = Black なら delta = +1)
        next.board.set(Coord::new(0, 0), Some(Color::Black));
        let r = DenseReward.compute(&s, &next, Color::Black, false);
        assert_eq!(r, 1.0);
    }
}
