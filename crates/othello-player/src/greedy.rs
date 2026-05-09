//! [`GreedyPlayer`]: 着手後の自分の石数最大化プレイヤー．

use crate::traits::{Player, PlayerError};
use othello_core::{Color, GameState, Move};

/// 着手後の自分の石数を最大化する手を選ぶ greedy プレイヤー．
///
/// 同点時は合法手リスト内の最初の手 ( 行優先 → 列優先で先に出るもの) を選ぶ．
/// パス以外の合法手がない場合は `Move::Pass` を返す．
pub struct GreedyPlayer {
    name: String,
    color: Color,
}

impl GreedyPlayer {
    /// 名前と色を指定して生成する．
    #[must_use]
    pub fn new(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            color,
        }
    }

    /// 標準名 `GreedyPlayer` で生成する．
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
