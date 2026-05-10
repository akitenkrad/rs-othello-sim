//! [`Color`] enum representing stone color.

use serde::{Deserialize, Serialize};

/// Stone color. Othello uses only black and white.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    /// Black stone (first player).
    Black,
    /// White stone (second player).
    White,
}

impl Color {
    /// Returns the opposing color: `Black ↔ White`.
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

    /// Returns a one-character display symbol. Black = `'X'`, White = `'O'`.
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
