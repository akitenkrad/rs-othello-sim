//! Hybrid [`Board`]. Uses [`Bitboard8`] for 8x8 and [`GenericBoard`] otherwise.

use crate::bitboard::Bitboard8;
use crate::color::Color;
use crate::coord::Coord;
use crate::error::OthelloError;
use crate::generic_board::GenericBoard;
use crate::mv::Move;
use serde::{Deserialize, Serialize};

/// Board dimensions (rows x columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoardSize {
    /// Number of rows (4..=26).
    pub rows: u8,
    /// Number of columns (4..=26).
    pub cols: u8,
}

impl BoardSize {
    /// Standard 8x8 size.
    pub const STANDARD: Self = Self { rows: 8, cols: 8 };

    /// Square board of `n × n`.
    #[inline]
    #[must_use]
    pub const fn square(n: u8) -> Self {
        Self { rows: n, cols: n }
    }
}

/// Hybrid board: bitboard for 8x8, generic implementation otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Board {
    /// 8x8-specific bitboard representation (fast).
    Bitboard8(Bitboard8),
    /// Generic N×M representation (flexible).
    Generic(GenericBoard),
}

impl Board {
    /// Constructs an empty board of the given size.
    ///
    /// Selects [`Bitboard8`] for 8x8 and [`GenericBoard`] otherwise.
    pub fn new(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows == 8 && size.cols == 8 {
            Ok(Self::Bitboard8(Bitboard8::empty()))
        } else {
            Ok(Self::Generic(GenericBoard::empty(size)?))
        }
    }

    /// Constructs a board of the given size with the **standard initial position**.
    pub fn standard(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows == 8 && size.cols == 8 {
            Ok(Self::Bitboard8(Bitboard8::standard()))
        } else {
            Ok(Self::Generic(GenericBoard::standard(size)?))
        }
    }

    /// Standard 8x8 initial position (shorthand).
    #[inline]
    #[must_use]
    pub fn standard_8x8() -> Self {
        Self::Bitboard8(Bitboard8::standard())
    }

    /// Returns the board size.
    #[inline]
    #[must_use]
    pub fn size(&self) -> BoardSize {
        match self {
            Self::Bitboard8(_) => BoardSize::STANDARD,
            Self::Generic(g) => g.size(),
        }
    }

    /// Returns the color at the given cell. `None` if the cell is empty or
    /// out of range.
    #[inline]
    #[must_use]
    pub fn cell(&self, coord: Coord) -> Option<Color> {
        match self {
            Self::Bitboard8(b) => b.cell(coord),
            Self::Generic(g) => g.cell(coord),
        }
    }

    /// Returns the number of stones of the given color.
    #[inline]
    #[must_use]
    pub fn count(&self, color: Color) -> u32 {
        match self {
            Self::Bitboard8(b) => b.count(color),
            Self::Generic(g) => g.count(color),
        }
    }

    /// Returns the number of empty cells.
    #[inline]
    #[must_use]
    pub fn empty_count(&self) -> u32 {
        match self {
            Self::Bitboard8(b) => b.empty_count(),
            Self::Generic(g) => g.empty_count(),
        }
    }

    /// Returns the legal moves for the given side.
    #[must_use]
    pub fn legal_moves(&self, side: Color) -> Vec<Move> {
        match self {
            Self::Bitboard8(b) => b.legal_moves(side),
            Self::Generic(g) => g.legal_moves(side),
        }
    }

    /// Whether at least one legal move exists.
    #[must_use]
    pub fn has_any_legal_move(&self, side: Color) -> bool {
        match self {
            Self::Bitboard8(b) => b.legal_mask(side) != 0,
            Self::Generic(g) => g.has_any_legal_move(side),
        }
    }

    /// Applies a move. On success, returns the coordinates of the flipped stones.
    pub fn apply(&mut self, side: Color, mv: Move) -> Result<Vec<Coord>, OthelloError> {
        match self {
            Self::Bitboard8(b) => b.apply(side, mv),
            Self::Generic(g) => g.apply(side, mv),
        }
    }

    /// Whether neither side has any legal moves (terminal state).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Bitboard8(b) => b.is_terminal(),
            Self::Generic(g) => g.is_terminal(),
        }
    }

    /// Returns the winner at the end of the game (the color with more
    /// stones). Returns `None` for a draw (equal stone counts).
    ///
    /// When called on a non-terminal state, this returns the currently
    /// leading color by stone count; use with care.
    #[must_use]
    pub fn winner(&self) -> Option<Color> {
        let b = self.count(Color::Black);
        let w = self.count(Color::White);
        match b.cmp(&w) {
            std::cmp::Ordering::Greater => Some(Color::Black),
            std::cmp::Ordering::Less => Some(Color::White),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// Test/setup helper that overwrites a cell directly.
    pub fn set(&mut self, coord: Coord, color: Option<Color>) {
        match self {
            Self::Bitboard8(b) => b.set(coord, color),
            Self::Generic(g) => g.set(coord, color),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_8x8_uses_bitboard() {
        let b = Board::standard(BoardSize::STANDARD).unwrap();
        assert!(matches!(b, Board::Bitboard8(_)));
    }

    #[test]
    fn standard_10x10_uses_generic() {
        let b = Board::standard(BoardSize::square(10)).unwrap();
        assert!(matches!(b, Board::Generic(_)));
    }

    #[test]
    fn winner_by_count() {
        let mut b = Board::standard_8x8();
        // 標準初期配置は 2 対 2 → None
        assert_eq!(b.winner(), None);
        // 黒石を追加
        b.set(Coord::new(0, 0), Some(Color::Black));
        assert_eq!(b.winner(), Some(Color::Black));
    }

    #[test]
    fn rejects_invalid_size() {
        let r = Board::new(BoardSize { rows: 3, cols: 8 });
        assert!(matches!(r, Err(OthelloError::InvalidBoardSize { .. })));
    }
}
