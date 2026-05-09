//! [`OthelloError`]: ライブラリ全体の Result 型で使われるエラー型．

use crate::coord::Coord;
use thiserror::Error;

/// `othello-core` で発生するエラー型．
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OthelloError {
    /// サポート外の盤面サイズが指定された．
    ///
    /// 許容範囲は $4 \times 4$ から $26 \times 26$ まで．
    #[error("invalid board size: rows={rows}, cols={cols} (allowed: 4..=26 for both dimensions)")]
    InvalidBoardSize {
        /// 行数．
        rows: u8,
        /// 列数．
        cols: u8,
    },

    /// 座標が盤面外を指している．
    #[error("coordinate out of bounds: ({row}, {col}) on {rows}x{cols} board")]
    OutOfBounds {
        /// 行．
        row: u8,
        /// 列．
        col: u8,
        /// 盤面行数．
        rows: u8,
        /// 盤面列数．
        cols: u8,
    },

    /// 不正な手 ( 既に石がある，1 つも反転しない，合法手があるのに Pass したなど)．
    #[error("illegal move at ({}, {}): {reason}", coord.row, coord.col)]
    IllegalMove {
        /// 不正手の座標．
        coord: Coord,
        /// 不正の理由．
        reason: IllegalMoveReason,
    },

    /// 合法手があるのに `Move::Pass` が指定された．
    #[error("illegal pass: legal moves exist for the side to move")]
    IllegalPass,
}

/// `IllegalMove` の詳細理由．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalMoveReason {
    /// 既に石が置かれているマス．
    Occupied,
    /// どの方向にも相手石を反転できない．
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
