//! 石の色を表す [`Color`] enum．

use serde::{Deserialize, Serialize};

/// 石の色．Othello には黒と白の 2 色のみ存在する．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    /// 黒石 ( 先手)．
    Black,
    /// 白石 ( 後手)．
    White,
}

impl Color {
    /// 相手の色を返す．`Black ↔ White`．
    ///
    /// ```
    /// use othello_core::Color;
    /// assert_eq!(Color::Black.opponent(), Color::White);
    /// assert_eq!(Color::White.opponent(), Color::Black);
    /// ```
    #[inline]
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }

    /// 表示用の 1 文字記号を返す．Black = `'X'`，White = `'O'`．
    #[inline]
    #[must_use]
    pub const fn glyph(self) -> char {
        match self {
            Self::Black => 'X',
            Self::White => 'O',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opponent_roundtrip() {
        assert_eq!(Color::Black.opponent().opponent(), Color::Black);
        assert_eq!(Color::White.opponent().opponent(), Color::White);
    }

    #[test]
    fn opponent_swaps() {
        assert_eq!(Color::Black.opponent(), Color::White);
        assert_eq!(Color::White.opponent(), Color::Black);
    }

    #[test]
    fn glyph_distinct() {
        assert_ne!(Color::Black.glyph(), Color::White.glyph());
        assert_eq!(Color::Black.glyph(), 'X');
        assert_eq!(Color::White.glyph(), 'O');
    }
}
