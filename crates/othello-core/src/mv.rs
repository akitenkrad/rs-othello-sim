//! [`Move`] enum representing a single move.

use crate::coord::Coord;
use serde::{Deserialize, Serialize};

/// A single move input: place a stone via `Place(Coord)` or pass via `Pass`.
///
/// Requesting `Pass` when legal moves exist is treated as an illegal move
/// and yields an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Move {
    /// Place a stone at the given coordinate.
    Place(Coord),
    /// Pass (legal only when there is no legal placement).
    Pass,
}

impl Move {
    /// Whether the move is a pass.
    #[inline]
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Returns the coordinate of `Place(coord)`, or `None` for `Pass`.
    #[inline]
    #[must_use]
    pub const fn coord(self) -> Option<Coord> {
        match self {
            Self::Place(c) => Some(c),
            Self::Pass => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_pass_works() {
        assert!(Move::Pass.is_pass());
        assert!(!Move::Place(Coord::new(0, 0)).is_pass());
    }

    #[test]
    fn coord_extraction() {
        assert_eq!(
            Move::Place(Coord::new(3, 4)).coord(),
            Some(Coord::new(3, 4))
        );
        assert_eq!(Move::Pass.coord(), None);
    }
}
