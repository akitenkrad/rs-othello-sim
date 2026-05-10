//! [`GreedyPlayer`]: a player that maximizes its own stones after the move.

use crate::traits::{Player, PlayerError};
use othello_core::{Color, GameState, Move};

/// Greedy player that picks the move maximizing the player's own stone
/// count after placing.
///
/// Ties are broken by taking the first move in the legal-moves list
/// (row-major, then column-major). When no non-pass legal move exists,
/// returns `Move::Pass`.
pub struct GreedyPlayer {
    name: String,
    color: Color,
}

impl GreedyPlayer {
    /// Constructs a player from a name and color.
    #[must_use]
    pub fn new(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            color,
        }
    }

    /// Constructs a player with the default name (`"GreedyPlayer"`).
    #[must_use]
    pub fn with_color(color: Color) -> Self {
        Self::new("GreedyPlayer", color)
    }
}

impl Player for GreedyPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color {
        self.color
    }

    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError> {
        let legal = state.legal_moves();
        if legal.is_empty() {
            return Ok(Move::Pass);
        }

        // 各候補手を試着し，自分の石数が最大の手を選ぶ
        let side = state.side_to_move;
        let mut best: Option<(Move, u32)> = None;
        for mv in legal {
            // クローンして仮想的に着手
            let mut probe = state.clone();
            // apply_move の失敗は想定外 ( legal_moves から得た手なので)
            if probe.apply_move(mv).is_err() {
                continue;
            }
            let count = probe.board.count(side);
            match best {
                None => best = Some((mv, count)),
                Some((_, b)) if count > b => best = Some((mv, count)),
                _ => {}
            }
        }

        best.map(|(m, _)| m).ok_or(PlayerError::IllegalMove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_legal_move() {
        let mut p = GreedyPlayer::with_color(Color::Black);
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        assert!(s.legal_moves().contains(&mv));
    }

    #[test]
    fn maximizes_own_stones_after_move() {
        let mut p = GreedyPlayer::with_color(Color::Black);
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();

        // 全合法手の中で自分の石数最大の手を選んでいるはず
        let legal = s.legal_moves();
        let mut max_count: u32 = 0;
        for cand in &legal {
            let mut probe = s.clone();
            probe.apply_move(*cand).unwrap();
            let c = probe.board.count(Color::Black);
            if c > max_count {
                max_count = c;
            }
        }
        let mut probe = s.clone();
        probe.apply_move(mv).unwrap();
        assert_eq!(probe.board.count(Color::Black), max_count);
    }
}
