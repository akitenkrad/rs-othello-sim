//! Arbitrary-sized generic board [`GenericBoard`].
//!
//! Supports board sizes from 4x4 up to 26x26. Cells are stored row-major in
//! a `Vec<Option<Color>>`.

use crate::board::BoardSize;
use crate::color::Color;
use crate::coord::Coord;
use crate::error::{IllegalMoveReason, OthelloError};
use crate::mv::Move;
use crate::rules::{flips_for_move, legal_move_coords};

/// Minimum allowed board side.
pub const MIN_SIDE: u8 = 4;
/// Maximum allowed board side (the upper bound representable with the
/// letters A through Z).
pub const MAX_SIDE: u8 = 26;

/// Arbitrary-sized board implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericBoard {
    size: BoardSize,
    /// Row-major (row 0 col 0, 1, ..., row 1 col 0, ...).
    cells: Vec<Option<Color>>,
}

impl GenericBoard {
    /// Constructs an empty board.
    ///
    /// Returns [`OthelloError::InvalidBoardSize`] if the size is out of the
    /// allowed range. The standard initial position requires both `rows`
    /// and `cols` to be even, but creating an empty board succeeds for any
    /// size within range.
    pub fn empty(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows < MIN_SIDE
            || size.rows > MAX_SIDE
            || size.cols < MIN_SIDE
            || size.cols > MAX_SIDE
        {
            return Err(OthelloError::InvalidBoardSize {
                rows: size.rows,
                cols: size.cols,
            });
        }
        let total = (size.rows as usize) * (size.cols as usize);
        Ok(Self {
            size,
            cells: vec![None; total],
        })
    }

    /// Standard Othello initial position (center four cells filled).
    ///
    /// Requires both `rows` and `cols` to be even. Returns
    /// [`OthelloError::InvalidBoardSize`] when either side is odd.
    pub fn standard(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows % 2 != 0 || size.cols % 2 != 0 {
            return Err(OthelloError::InvalidBoardSize {
                rows: size.rows,
                cols: size.cols,
            });
        }
        let mut b = Self::empty(size)?;
        let r_mid = size.rows / 2;
        let c_mid = size.cols / 2;
        // (r_mid - 1, c_mid - 1) = White
        // (r_mid - 1, c_mid)     = Black
        // (r_mid,     c_mid - 1) = Black
        // (r_mid,     c_mid)     = White
        b.set_unchecked(Coord::new(r_mid - 1, c_mid - 1), Some(Color::White));
        b.set_unchecked(Coord::new(r_mid - 1, c_mid), Some(Color::Black));
        b.set_unchecked(Coord::new(r_mid, c_mid - 1), Some(Color::Black));
        b.set_unchecked(Coord::new(r_mid, c_mid), Some(Color::White));
        Ok(b)
    }

    /// Returns the board size.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> BoardSize {
        self.size
    }

    #[inline]
    fn index(&self, coord: Coord) -> Option<usize> {
        if coord.row >= self.size.rows || coord.col >= self.size.cols {
            None
        } else {
            Some((coord.row as usize) * (self.size.cols as usize) + (coord.col as usize))
        }
    }

    /// Returns the color at the given cell. `None` if empty or out of range.
    #[inline]
    #[must_use]
    pub fn cell(&self, coord: Coord) -> Option<Color> {
        self.index(coord).and_then(|i| self.cells[i])
    }

    /// Overwrites a cell (out-of-range coordinates are silently ignored).
    /// Intended for tests and setup.
    pub fn set(&mut self, coord: Coord, color: Option<Color>) {
        if let Some(i) = self.index(coord) {
            self.cells[i] = color;
        }
    }

    /// Internal `set` that assumes the coordinate has already been
    /// range-checked.
    fn set_unchecked(&mut self, coord: Coord, color: Option<Color>) {
        let i = (coord.row as usize) * (self.size.cols as usize) + (coord.col as usize);
        self.cells[i] = color;
    }

    /// Returns the number of stones of the given color.
    #[must_use]
    pub fn count(&self, color: Color) -> u32 {
        self.cells
            .iter()
            .filter(|c| **c == Some(color))
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    /// Returns the number of empty cells.
    #[must_use]
    pub fn empty_count(&self) -> u32 {
        self.cells
            .iter()
            .filter(|c| c.is_none())
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    /// Returns the legal moves for the given side.
    #[must_use]
    pub fn legal_moves(&self, side: Color) -> Vec<Move> {
        let cell_at = |r: u8, c: u8| self.cell(Coord::new(r, c));
        legal_move_coords(side, self.size.rows, self.size.cols, cell_at)
            .into_iter()
            .map(Move::Place)
            .collect()
    }

    /// Applies a move. Returns the coordinates of the flipped stones (empty
    /// for `Pass`).
    pub fn apply(&mut self, side: Color, mv: Move) -> Result<Vec<Coord>, OthelloError> {
        match mv {
            Move::Pass => {
                if self.has_any_legal_move(side) {
                    Err(OthelloError::IllegalPass)
                } else {
                    Ok(Vec::new())
                }
            }
            Move::Place(coord) => {
                if coord.row >= self.size.rows || coord.col >= self.size.cols {
                    return Err(OthelloError::OutOfBounds {
                        row: coord.row,
                        col: coord.col,
                        rows: self.size.rows,
                        cols: self.size.cols,
                    });
                }
                if self.cell(coord).is_some() {
                    return Err(OthelloError::IllegalMove {
                        coord,
                        reason: IllegalMoveReason::Occupied,
                    });
                }
                let cell_at = |r: u8, c: u8| self.cell(Coord::new(r, c));
                let flips = flips_for_move(side, coord, self.size.rows, self.size.cols, cell_at);
                if flips.is_empty() {
                    return Err(OthelloError::IllegalMove {
                        coord,
                        reason: IllegalMoveReason::NoFlips,
                    });
                }
                // 自石を置き，反転を反映
                self.set_unchecked(coord, Some(side));
                for f in &flips {
                    self.set_unchecked(*f, Some(side));
                }
                Ok(flips)
            }
        }
    }

    /// Whether at least one legal move exists (with early exit).
    #[must_use]
    pub fn has_any_legal_move(&self, side: Color) -> bool {
        // legal_moves と同じロジックだが Vec を作らずに回す
        for r in 0..self.size.rows {
            for c in 0..self.size.cols {
                if self.cell(Coord::new(r, c)).is_some() {
                    continue;
                }
                let cell_at = |rr: u8, cc: u8| self.cell(Coord::new(rr, cc));
                let flips = flips_for_move(
                    side,
                    Coord::new(r, c),
                    self.size.rows,
                    self.size.cols,
                    cell_at,
                );
                if !flips.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Whether neither side has any legal moves (terminal state).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        !self.has_any_legal_move(Color::Black) && !self.has_any_legal_move(Color::White)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord_set(coords: &[(u8, u8)]) -> std::collections::BTreeSet<Coord> {
        coords.iter().map(|&(r, c)| Coord::new(r, c)).collect()
    }

    fn moves_to_coord_set(moves: &[Move]) -> std::collections::BTreeSet<Coord> {
        moves.iter().filter_map(|m| m.coord()).collect()
    }

    #[test]
    fn rejects_too_small() {
        let r = GenericBoard::empty(BoardSize { rows: 2, cols: 8 });
        assert!(matches!(r, Err(OthelloError::InvalidBoardSize { .. })));
    }

    #[test]
    fn rejects_too_large() {
        let r = GenericBoard::empty(BoardSize { rows: 27, cols: 27 });
        assert!(matches!(r, Err(OthelloError::InvalidBoardSize { .. })));
    }

    #[test]
    fn standard_4x4() {
        let b = GenericBoard::standard(BoardSize { rows: 4, cols: 4 }).unwrap();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        assert_eq!(b.cell(Coord::new(1, 1)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(1, 2)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(2, 1)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(2, 2)), Some(Color::White));
    }

    #[test]
    fn standard_6x6() {
        let b = GenericBoard::standard(BoardSize { rows: 6, cols: 6 }).unwrap();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        assert_eq!(b.cell(Coord::new(2, 2)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(3, 3)), Some(Color::White));
    }

    #[test]
    fn standard_8x8() {
        let b = GenericBoard::standard(BoardSize { rows: 8, cols: 8 }).unwrap();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        // 黒の初期合法手 4 つ
        let moves = b.legal_moves(Color::Black);
        let expected = coord_set(&[(2, 3), (3, 2), (4, 5), (5, 4)]);
        assert_eq!(moves_to_coord_set(&moves), expected);
    }

    #[test]
    fn standard_10x10() {
        let b = GenericBoard::standard(BoardSize { rows: 10, cols: 10 }).unwrap();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        // 中央 4 マス
        assert_eq!(b.cell(Coord::new(4, 4)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(4, 5)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(5, 4)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(5, 5)), Some(Color::White));
        // 黒の合法手 4 つ
        let moves = b.legal_moves(Color::Black);
        let expected = coord_set(&[(3, 4), (4, 3), (5, 6), (6, 5)]);
        assert_eq!(moves_to_coord_set(&moves), expected);
    }

    #[test]
    fn standard_26x26() {
        let b = GenericBoard::standard(BoardSize { rows: 26, cols: 26 }).unwrap();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        // 中央 4 マスの座標を確認
        assert_eq!(b.cell(Coord::new(12, 12)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(12, 13)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(13, 12)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(13, 13)), Some(Color::White));
    }

    #[test]
    fn rejects_odd_dimensions_for_standard() {
        let r = GenericBoard::standard(BoardSize { rows: 5, cols: 6 });
        assert!(matches!(r, Err(OthelloError::InvalidBoardSize { .. })));
    }

    #[test]
    fn apply_first_black_move_8x8() {
        let mut b = GenericBoard::standard(BoardSize { rows: 8, cols: 8 }).unwrap();
        let flipped = b
            .apply(Color::Black, Move::Place(Coord::new(2, 3)))
            .unwrap();
        assert_eq!(flipped, vec![Coord::new(3, 3)]);
        assert_eq!(b.cell(Coord::new(2, 3)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(3, 3)), Some(Color::Black));
    }

    #[test]
    fn illegal_move_no_flips_returns_error() {
        let mut b = GenericBoard::standard(BoardSize { rows: 8, cols: 8 }).unwrap();
        let err = b
            .apply(Color::Black, Move::Place(Coord::new(0, 0)))
            .unwrap_err();
        assert!(matches!(
            err,
            OthelloError::IllegalMove {
                reason: IllegalMoveReason::NoFlips,
                ..
            }
        ));
    }

    #[test]
    fn illegal_pass_when_moves_exist() {
        let mut b = GenericBoard::standard(BoardSize { rows: 8, cols: 8 }).unwrap();
        let err = b.apply(Color::Black, Move::Pass).unwrap_err();
        assert_eq!(err, OthelloError::IllegalPass);
    }

    #[test]
    fn terminal_when_full() {
        let mut b = GenericBoard::empty(BoardSize { rows: 4, cols: 4 }).unwrap();
        for r in 0..4u8 {
            for c in 0..4u8 {
                b.set(Coord::new(r, c), Some(Color::Black));
            }
        }
        assert!(b.is_terminal());
    }
}
