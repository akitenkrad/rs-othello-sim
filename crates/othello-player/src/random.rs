//! [`RandomPlayer`]: 合法手から一様サンプリングするプレイヤー．

use crate::traits::{Player, PlayerError};
use othello_core::{Color, GameState, Move};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

/// 合法手から `ChaCha8Rng` で一様にサンプリングする決定的プレイヤー．
///
/// 同じ seed と同じ局面列なら同じ手列を返す．
pub struct RandomPlayer {
    name: String,
    color: Color,
    rng: ChaCha8Rng,
    initial_seed: u64,
}

impl RandomPlayer {
    /// 名前 + 色 + seed を指定して生成する．
    #[must_use]
    pub fn new(name: impl Into<String>, color: Color, seed: u64) -> Self {
        Self {
            name: name.into(),
            color,
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
        }
    }

    /// `RandomPlayer` という標準名で seed のみ指定して生成する．
    #[must_use]
    pub fn with_seed(color: Color, seed: u64) -> Self {
        Self::new("RandomPlayer", color, seed)
    }

    /// 初期 seed を返す ( ロギング・記録用)．
    #[inline]
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.initial_seed
    }
}

impl Player for RandomPlayer {
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
        // 合法手から一様サンプル
        let mv = legal.choose(&mut self.rng).copied().ok_or_else(|| {
            PlayerError::Other("RandomPlayer: failed to sample legal move".into())
        })?;
        Ok(mv)
    }

    fn reset(&mut self) {
        self.rng = ChaCha8Rng::seed_from_u64(self.initial_seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_legal_move() {
        let mut p = RandomPlayer::with_seed(Color::Black, 42);
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        let legal = s.legal_moves();
        assert!(legal.contains(&mv));
    }

    #[test]
    fn deterministic_with_same_seed() {
        let mut p1 = RandomPlayer::with_seed(Color::Black, 7);
        let mut p2 = RandomPlayer::with_seed(Color::Black, 7);
        let s = GameState::standard_8x8();
        for _ in 0..10 {
            assert_eq!(p1.select_move(&s).unwrap(), p2.select_move(&s).unwrap());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        // ある程度のサンプル数で違いが現れる
        let mut p1 = RandomPlayer::with_seed(Color::Black, 1);
        let mut p2 = RandomPlayer::with_seed(Color::Black, 99);
        let s = GameState::standard_8x8();
        let mut diff = false;
        for _ in 0..50 {
            if p1.select_move(&s).unwrap() != p2.select_move(&s).unwrap() {
                diff = true;
                break;
            }
        }
        assert!(diff, "seeds 1 and 99 should diverge within 50 samples");
    }

    #[test]
    fn reset_restores_sequence() {
        let mut p = RandomPlayer::with_seed(Color::Black, 7);
        let s = GameState::standard_8x8();
        let m1 = p.select_move(&s).unwrap();
        p.reset();
        let m2 = p.select_move(&s).unwrap();
        assert_eq!(m1, m2);
    }
}
