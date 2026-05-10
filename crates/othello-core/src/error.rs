//! [`OthelloError`]: error type used by the library-wide `Result`.

use crate::coord::Coord;
use thiserror::Error;

/// Error type produced by `othello-core`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OthelloError {
    /// An unsupported board size was requested.
    ///
    /// Allowed range is from $4 \times 4$ to $26 \times 26$.
    #[error("invalid board size: rows={rows}, cols={cols} (allowed: 4..=26 for both dimensions)")]
    InvalidBoardSize {
        /// Number of rows.
        rows: u8,
        /// Number of columns.
        cols: u8,
    },

    /// A coordinate falls outside the board.
    #[error("coordinate out of bounds: ({row}, {col}) on {rows}x{cols} board")]
    OutOfBounds {
        /// Row.
        row: u8,
        /// Column.
        col: u8,
        /// Board rows.
        rows: u8,
        /// Board columns.
        cols: u8,
    },

    /// Illegal move (cell already occupied, no flipped stones, pass requested
    /// while legal moves exist, etc.).
    #[error("illegal move at ({}, {}): {reason}", coord.row, coord.col)]
    IllegalMove {
        /// Coordinate of the illegal move.
        coord: Coord,
        /// Reason the move is illegal.
        reason: IllegalMoveReason,
    },

    /// `Move::Pass` was requested while legal moves exist.
    #[error("illegal pass: legal moves exist for the side to move")]
    IllegalPass,
}

/// Detailed reason for an `IllegalMove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalMoveReason {
    /// The cell is already occupied.
    Occupied,
    /// The move flips no opponent stones in any direction.
    NoFlips,
}

impl std::fmt::Display for IllegalMoveReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied => f.write_str("cell is already occupied"),
            Self::NoFlips => f.write_str("move flips no opponent stones"),
        }
    }
}
