//! [`Action`] space and conversion between `Move` and `Action`.
//!
//! The action space has size `H * W + 1`, where the final entry encodes
//! `Pass`.

use crate::error::RlError;
use othello_core::{BoardSize, Coord, Move};
use serde::{Deserialize, Serialize};

/// Discrete action of a reinforcement-learning agent.
///
/// `0..H*W` indexes a cell on the board (`index = row * cols + col`); the
/// value `H*W` represents `Pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Action(pub u32);

impl Action {
    /// Converts the given `Move` into an `Action`.
    #[must_use]
    pub fn from_move(mv: Move, size: BoardSize) -> Self {
        match mv {
            Move::Place(c) => Self(c.row as u32 * size.cols as u32 + c.col as u32),
            Move::Pass => Self(size.rows as u32 * size.cols as u32),
        }
    }

    /// Converts the action into a `Move`. Indices beyond `size` are treated
    /// as `Move::Pass`.
    #[must_use]
    pub fn to_move(self, size: BoardSize) -> Move {
        let board_cells = size.rows as u32 * size.cols as u32;
        if self.0 >= board_cells {
            Move::Pass
        } else {
            let row = (self.0 / size.cols as u32) as u8;
            let col = (self.0 % size.cols as u32) as u8;
            Move::Place(Coord::new(row, col))
        }
    }

    /// Returns the action value representing `Pass`.
    #[inline]
    #[must_use]
    pub fn pass(size: BoardSize) -> Self {
        Self(size.rows as u32 * size.cols as u32)
    }

    /// Returns the action-space size (`H * W + 1`).
    #[inline]
    #[must_use]
    pub fn space_size(size: BoardSize) -> u32 {
        size.rows as u32 * size.cols as u32 + 1
    }

    /// Range-checks the action, returning [`RlError::OutOfRange`] when
    /// invalid.
    pub fn validate(self, size: BoardSize) -> Result<(), RlError> {
        let max = Self::space_size(size);
        if self.0 >= max {
            Err(RlError::OutOfRange {
                action: self.0,
                max,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_roundtrip_8x8() {
        let size = BoardSize::STANDARD;
        for r in 0..8u8 {
            for c in 0..8u8 {
                let mv = Move::Place(Coord::new(r, c));
                let a = Action::from_move(mv, size);
                assert_eq!(a.to_move(size), mv);
            }
        }
    }

    #[test]
    fn pass_roundtrip() {
        let size = BoardSize::STANDARD;
        let mv = Move::Pass;
        let a = Action::from_move(mv, size);
        assert_eq!(a, Action::pass(size));
        assert_eq!(a.to_move(size), Move::Pass);
    }

    #[test]
    fn space_size_8x8() {
        assert_eq!(Action::space_size(BoardSize::STANDARD), 65);
    }

    #[test]
    fn space_size_4x4() {
        assert_eq!(Action::space_size(BoardSize::square(4)), 17);
    }

    #[test]
    fn validate_in_range() {
        let size = BoardSize::STANDARD;
        Action(0).validate(size).unwrap();
        Action(64).validate(size).unwrap();
    }

    #[test]
    fn validate_out_of_range() {
        let size = BoardSize::STANDARD;
        assert!(Action(65).validate(size).is_err());
        assert!(Action(1000).validate(size).is_err());
    }
}
