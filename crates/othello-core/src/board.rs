//! ハイブリッド盤面 [`Board`]．8×8 では [`Bitboard8`]，それ以外は [`GenericBoard`] を使用する．

use crate::bitboard::Bitboard8;
use crate::color::Color;
use crate::coord::Coord;
use crate::error::OthelloError;
use crate::generic_board::GenericBoard;
use crate::mv::Move;
use serde::{Deserialize, Serialize};

/// 盤面サイズ ( 行数 × 列数)．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoardSize {
    /// 行数 ( 4..=26)．
    pub rows: u8,
    /// 列数 ( 4..=26)．
    pub cols: u8,
}

impl BoardSize {
    /// 標準的な 8×8 サイズ．
    pub const STANDARD: Self = Self { rows: 8, cols: 8 };

    /// `n × n` の正方形盤面．
    #[inline]
    #[must_use]
    pub const fn square(n: u8) -> Self {
        Self { rows: n, cols: n }
    }
}

/// ハイブリッド盤面．8×8 のときは bitboard，それ以外は汎用実装が選ばれる．
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Board {
    /// 8×8 専用 bitboard 表現 ( 高速)．
    Bitboard8(Bitboard8),
    /// 任意 N×M サイズ ( 柔軟)．
    Generic(GenericBoard),
}

impl Board {
    /// 指定サイズの空盤面を生成する．
    ///
    /// 8×8 のときは [`Bitboard8`]，それ以外は [`GenericBoard`] を選択する．
    pub fn new(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows == 8 && size.cols == 8 {
            Ok(Self::Bitboard8(Bitboard8::empty()))
        } else {
            Ok(Self::Generic(GenericBoard::empty(size)?))
        }
    }

    /// 指定サイズの **標準初期配置** 盤面を生成する．
    pub fn standard(size: BoardSize) -> Result<Self, OthelloError> {
        if size.rows == 8 && size.cols == 8 {
            Ok(Self::Bitboard8(Bitboard8::standard()))
        } else {
            Ok(Self::Generic(GenericBoard::standard(size)?))
        }
    }

    /// 標準 8×8 初期配置 ( shorthand)．
    #[inline]
    #[must_use]
    pub fn standard_8x8() -> Self {
        Self::Bitboard8(Bitboard8::standard())
    }

    /// 盤面サイズを返す．
    #[inline]
    #[must_use]
    pub fn size(&self) -> BoardSize {
        match self {
            Self::Bitboard8(_) => BoardSize::STANDARD,
            Self::Generic(g) => g.size(),
        }
    }

    /// 指定マスの色を返す．空マスや範囲外は `None`．
    #[inline]
    #[must_use]
    pub fn cell(&self, coord: Coord) -> Option<Color> {
        match self {
            Self::Bitboard8(b) => b.cell(coord),
            Self::Generic(g) => g.cell(coord),
        }
    }

    /// 指定色の石数を返す．
    #[inline]
    #[must_use]
    pub fn count(&self, color: Color) -> u32 {
        match self {
            Self::Bitboard8(b) => b.count(color),
            Self::Generic(g) => g.count(color),
        }
    }

    /// 空マス数を返す．
    #[inline]
    #[must_use]
    pub fn empty_count(&self) -> u32 {
        match self {
            Self::Bitboard8(b) => b.empty_count(),
            Self::Generic(g) => g.empty_count(),
        }
    }

    /// 指定色の合法手リストを返す．
    #[must_use]
    pub fn legal_moves(&self, side: Color) -> Vec<Move> {
        match self {
            Self::Bitboard8(b) => b.legal_moves(side),
            Self::Generic(g) => g.legal_moves(side),
        }
    }

    /// 合法手が 1 つでも存在するかどうか．
    #[must_use]
    pub fn has_any_legal_move(&self, side: Color) -> bool {
        match self {
            Self::Bitboard8(b) => b.legal_mask(side) != 0,
            Self::Generic(g) => g.has_any_legal_move(side),
        }
    }

    /// 着手を適用する．成功時は反転した石の座標 Vec を返す．
    pub fn apply(&mut self, side: Color, mv: Move) -> Result<Vec<Coord>, OthelloError> {
        match self {
            Self::Bitboard8(b) => b.apply(side, mv),
            Self::Generic(g) => g.apply(side, mv),
        }
    }

    /// 両者とも合法手なし ( 終局) かどうか．
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Bitboard8(b) => b.is_terminal(),
            Self::Generic(g) => g.is_terminal(),
        }
    }

    /// 終局時の勝者を返す．石数が多い色．同数なら `None` ( 引き分け)．
    ///
    /// 終局でない場合も石数比較で「現時点での優勢色」を返す ( 慎重に使うこと)．
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

    /// テスト・初期化用にマスを直接書き換える．
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
