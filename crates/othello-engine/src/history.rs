//! [`GameHistory`]: per-move history stored as full snapshots.

use othello_core::{GameState, Move};

/// History of a single game. Uses **full snapshots** — keeps the initial
/// state plus the state after each move ($n+1$ states in total).
#[derive(Debug, Clone)]
pub struct GameHistory {
    snapshots: Vec<GameState>,
    moves: Vec<Move>,
}

impl GameHistory {
    /// Builds an empty history that only contains the initial state.
    #[must_use]
    pub fn new(initial: GameState) -> Self {
        Self {
            snapshots: vec![initial],
            moves: Vec::new(),
        }
    }

    /// Appends a single move and the resulting state.
    pub fn push(&mut self, mv: Move, post_state: GameState) {
        self.moves.push(mv);
        self.snapshots.push(post_state);
    }

    /// Returns the initial state.
    #[inline]
    #[must_use]
    pub fn initial(&self) -> &GameState {
        &self.snapshots[0]
    }

    /// Returns all snapshots (length = number of moves + 1).
    #[inline]
    #[must_use]
    pub fn snapshots(&self) -> &[GameState] {
        &self.snapshots
    }

    /// Returns the move sequence (length = number of moves).
    #[inline]
    #[must_use]
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Total number of moves (includes passes).
    #[inline]
    #[must_use]
    pub fn total_moves(&self) -> usize {
        self.moves.len()
    }

    /// Returns the snapshot at `index` (0 = initial state, `n` = state
    /// after the `n`-th move).
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
