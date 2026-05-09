//! [`GameHistory`]: スナップショット方式で全手の状態を保持する履歴．

use othello_core::{GameState, Move};

/// 1 局分の履歴．**完全スナップショット方式**で，初期状態 + 各手後の状態 ( $n+1$ 個) を保持する．
#[derive(Debug, Clone)]
pub struct GameHistory {
    snapshots: Vec<GameState>,
    moves: Vec<Move>,
}

impl GameHistory {
    /// 初期状態のみを持つ空履歴を生成する．
    #[must_use]
    pub fn new(initial: GameState) -> Self {
        Self {
            snapshots: vec![initial],
            moves: Vec::new(),
        }
    }

    /// 着手後の状態を 1 ステップ追加する．
    pub fn push(&mut self, mv: Move, post_state: GameState) {
        self.moves.push(mv);
        self.snapshots.push(post_state);
    }

    /// 初期状態を返す．
    #[inline]
    #[must_use]
    pub fn initial(&self) -> &GameState {
        &self.snapshots[0]
    }

    /// 全スナップショット ( 長さは手数 + 1)．
    #[inline]
    #[must_use]
    pub fn snapshots(&self) -> &[GameState] {
        &self.snapshots
    }

    /// 着手列 ( 長さは手数)．
    #[inline]
    #[must_use]
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// 総手数 ( パス含む)．
    #[inline]
    #[must_use]
    pub fn total_moves(&self) -> usize {
        self.moves.len()
    }

    /// `index` 手目時点 ( 0 = 初期状態，`n` = `n` 手目適用後) のスナップショットを返す．
    #[inline]
    #[must_use]
    pub fn snapshot_at(&self, index: usize) -> Option<&GameState> {
        self.snapshots.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::prelude::*;

    #[test]
    fn new_history_has_only_initial() {
        let s = GameState::standard_8x8();
        let h = GameHistory::new(s.clone());
        assert_eq!(h.snapshots().len(), 1);
        assert_eq!(h.moves().len(), 0);
        assert_eq!(h.initial(), &s);
    }

    #[test]
    fn push_records_state() {
        let mut s = GameState::standard_8x8();
        let mut h = GameHistory::new(s.clone());
        let mv = Move::Place(Coord::new(2, 3));
        s.apply_move(mv).unwrap();
        h.push(mv, s.clone());
        assert_eq!(h.snapshots().len(), 2);
        assert_eq!(h.moves(), &[mv]);
        assert_eq!(h.snapshot_at(1).unwrap(), &s);
    }
}
