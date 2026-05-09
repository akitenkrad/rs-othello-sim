//! 盤面上の座標を表す [`Coord`] 型．

use serde::{Deserialize, Serialize};

/// 盤面上の座標．`row` と `col` はゼロ起点．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Coord {
    /// 行 ( 0 起点)．
    pub row: u8,
    /// 列 ( 0 起点)．
    pub col: u8,
}

impl Coord {
    /// 新しい `Coord` を生成する．境界チェックは行わない ( ライブラリ呼び出し側の責任)．
    #[inline]
    #[must_use]
    pub const fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }

    /// 8×8 bitboard における bit index を返す ( `row * 8 + col`)．
    ///
    /// 8×8 以外の盤面では使用しないこと．
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
