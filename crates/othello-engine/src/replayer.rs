//! [`Replayer`]: `GameHistory` 上で前後移動・任意手数ジャンプを行う．

use crate::history::GameHistory;
use othello_core::GameState;

/// 履歴上のカーソル ( = 現在表示している手数 0..=`total_moves`)．
///
/// `cursor = 0` は初期状態，`cursor = n` は $n$ 手目適用後を指す．
#[derive(Debug)]
pub struct Replayer<'a> {
    history: &'a GameHistory,
    cursor: usize,
}

impl<'a> Replayer<'a> {
    /// `GameHistory` の参照から新規 Replayer を生成する．カーソルは初期状態 ( 0)．
    #[must_use]
    pub fn new(history: &'a GameHistory) -> Self {
        Self { history, cursor: 0 }
    }

    /// 現在のカーソル位置．
    #[inline]
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 履歴中の総手数．
    #[inline]
    #[must_use]
    pub fn total_moves(&self) -> usize {
        self.history.total_moves()
    }

    /// 現在の `GameState`．
    #[must_use]
    pub fn current(&self) -> &GameState {
        // GameHistory は最低でも初期状態 1 個を持つので unwrap は安全
        self.history.snapshot_at(self.cursor).unwrap()
    }

    /// 1 手進める．既に最後尾なら `None`．
    pub fn step_forward(&mut self) -> Option<&GameState> {
        if self.cursor < self.total_moves() {
            self.cursor += 1;
            self.history.snapshot_at(self.cursor)
        } else {
            None
        }
    }

    /// 1 手戻す．既に初期状態なら `None`．
    pub fn step_backward(&mut self) -> Option<&GameState> {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.history.snapshot_at(self.cursor)
        } else {
            None
        }
    }

    /// 指定手数にジャンプする．`move_number` は 0..=`total_moves` の範囲外なら `None`．
    pub fn jump_to(&mut self, move_number: usize) -> Option<&GameState> {
        if move_number > self.total_moves() {
            return None;
        }
        self.cursor = move_number;
        self.history.snapshot_at(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::prelude::*;

    fn build_history() -> GameHistory {
        let mut s = GameState::standard_8x8();
        let mut h = GameHistory::new(s.clone());
        for mv in [
            Move::Place(Coord::new(2, 3)),
            Move::Place(Coord::new(2, 2)),
            Move::Place(Coord::new(3, 2)),
        ] {
            s.apply_move(mv).unwrap();
            h.push(mv, s.clone());
        }
        h
    }

    #[test]
    fn step_forward_then_back_returns_initial() {
        let h = build_history();
        let mut r = Replayer::new(&h);
        for _ in 0..h.total_moves() {
            assert!(r.step_forward().is_some());
        }
        assert_eq!(r.cursor(), h.total_moves());
        for _ in 0..h.total_moves() {
            assert!(r.step_backward().is_some());
        }
        assert_eq!(r.cursor(), 0);
        assert_eq!(r.current(), h.initial());
    }

    #[test]
    fn jump_to_matches_step_forward() {
        let h = build_history();
        for n in 0..=h.total_moves() {
            let mut r1 = Replayer::new(&h);
            r1.jump_to(n).unwrap();

            let mut r2 = Replayer::new(&h);
            for _ in 0..n {
                r2.step_forward();
            }

            assert_eq!(r1.current(), r2.current());
        }
    }

    #[test]
    fn out_of_range_jump_returns_none() {
        let h = build_history();
        let mut r = Replayer::new(&h);
        assert!(r.jump_to(h.total_moves() + 1).is_none());
    }
}
