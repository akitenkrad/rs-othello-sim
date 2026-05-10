//! [`Coord`] type representing a coordinate on the board.

use serde::{Deserialize, Serialize};

/// Coordinate on the board. `row` and `col` are zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Coord {
    /// Row (0-based).
    pub row: u8,
    /// Column (0-based).
    pub col: u8,
}

impl Coord {
    /// Constructs a new `Coord`. No bounds checking is performed; the caller
    /// is responsible for ensuring the coordinate is within range.
    #[inline]
    #[must_use]
    pub const fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }

    /// Returns the bit index in an 8x8 bitboard (`row * 8 + col`).
    ///
    /// Must not be used with non-8x8 boards.
    #[inline]
    #[must_use]
    pub const fn to_bit_index_8x8(self) -> u32 {
        (self.row as u32) * 8 + (self.col as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_components() {
        let c = Coord::new(3, 4);
        assert_eq!(c.row, 3);
        assert_eq!(c.col, 4);
    }

    #[test]
    fn bit_index_corners() {
        assert_eq!(Coord::new(0, 0).to_bit_index_8x8(), 0);
        assert_eq!(Coord::new(0, 7).to_bit_index_8x8(), 7);
        assert_eq!(Coord::new(7, 0).to_bit_index_8x8(), 56);
        assert_eq!(Coord::new(7, 7).to_bit_index_8x8(), 63);
    }

    #[test]
    fn equality_and_ordering() {
        assert_eq!(Coord::new(2, 3), Coord::new(2, 3));
        assert!(Coord::new(2, 3) < Coord::new(2, 4));
        assert!(Coord::new(2, 7) < Coord::new(3, 0));
    }
}
