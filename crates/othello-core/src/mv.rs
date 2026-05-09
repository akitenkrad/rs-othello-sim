//! 1 手を表す [`Move`] enum．

use crate::coord::Coord;
use serde::{Deserialize, Serialize};

/// 1 手の入力．石を置く `Place(Coord)` か，パスする `Pass`．
///
/// 合法手が存在するときに `Pass` を要求した場合は不正手としてエラーになる．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Move {
    /// 指定座標に石を置く．
    Place(Coord),
    /// パスする ( 合法手がない場合のみ正当)．
    Pass,
}

impl Move {
    /// パスかどうか．
    #[inline]
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }

    /// `Place(coord)` の座標を取り出す．`Pass` の場合は `None`．
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
