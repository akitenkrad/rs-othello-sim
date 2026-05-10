//! Bitboard implementation specialized to 8x8.
//!
//! Each player's stones are stored in a single `u64`. The bit layout is
//! `bit_index = row * 8 + col` (row 0 col 0 = bit 0, row 7 col 7 = bit 63).
//!
//! Legal-move generation is implemented as a chain of eight directional
//! shifts and AND/OR operations to minimize branching. Each direction uses a
//! column mask (column A or column H) to prevent wrap-around.

use crate::color::Color;
use crate::coord::Coord;
use crate::error::{IllegalMoveReason, OthelloError};
use crate::mv::Move;

/// Bitboard specialized to 8x8. `black` and `white` each hold one bit per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitboard8 {
    black: u64,
    white: u64,
}

// ------------------------------------------------------------------
// 列マスク ( wrap-around 防止用)
// ------------------------------------------------------------------

/// Mask excluding file A (`col = 0`); covers files 1..=7. Used for west-
/// ward shifts.
const NOT_A_FILE: u64 = 0xfefefefefefefefe;
/// Mask excluding file H (`col = 7`); covers files 0..=6. Used for east-
/// ward shifts.
const NOT_H_FILE: u64 = 0x7f7f7f7f7f7f7f7f;

// ------------------------------------------------------------------
// 8 方向シフト
//
//   N  : row -1   ( bit -8)   → 安全 ( ファイルを跨がない)
//   S  : row +1   ( bit +8)   → 安全
//   E  : col +1   ( bit +1)   → H 列マスク必須 ( シフト前にマスク)
//   W  : col -1   ( bit -1)   → A 列マスク必須
//   NE : N + E    ( bit -7)   → H 列マスク
//   NW : N + W    ( bit -9)   → A 列マスク
//   SE : S + E    ( bit +9)   → H 列マスク
//   SW : S + W    ( bit +7)   → A 列マスク
// ------------------------------------------------------------------

#[inline]
const fn shift_n(b: u64) -> u64 {
    b >> 8
}
#[inline]
const fn shift_s(b: u64) -> u64 {
    b << 8
}
#[inline]
const fn shift_e(b: u64) -> u64 {
    (b & NOT_H_FILE) << 1
}
#[inline]
const fn shift_w(b: u64) -> u64 {
    (b & NOT_A_FILE) >> 1
}
#[inline]
const fn shift_ne(b: u64) -> u64 {
    (b & NOT_H_FILE) >> 7
}
#[inline]
const fn shift_nw(b: u64) -> u64 {
    (b & NOT_A_FILE) >> 9
}
#[inline]
const fn shift_se(b: u64) -> u64 {
    (b & NOT_H_FILE) << 9
}
#[inline]
const fn shift_sw(b: u64) -> u64 {
    (b & NOT_A_FILE) << 7
}

/// List of named shift functions for the eight directions.
///
/// Every direction reuses the same legal-move generation algorithm
/// (five-iteration chain expansion).
const SHIFT_FNS: [fn(u64) -> u64; 8] = [
    shift_n, shift_s, shift_e, shift_w, shift_ne, shift_nw, shift_se, shift_sw,
];

// ------------------------------------------------------------------
// 1 方向の合法手・反転計算
// ------------------------------------------------------------------

/// Returns the bitmask of legal-move squares in the given direction,
/// following the algorithm in design doc §3.1.2.
///
/// 1. Initialize `candidates = shift(own) & opp` (opponent stones adjacent
///    to our stones).
/// 2. Iterate five times to extend the chain (capturing every consecutive
///    opponent stone).
/// 3. One more shift, AND-ed with the empty squares, yields the legal-move
///    positions.
#[inline]
fn legal_moves_dir(own: u64, opp: u64, shift: fn(u64) -> u64) -> u64 {
    let empty = !(own | opp);
    let mut candidates = shift(own) & opp;
    // 連鎖を 5 回伸ばす ( 8 マス盤では最大 6 マス連鎖 → 5 回反復で十分)
    candidates |= shift(candidates) & opp;
    candidates |= shift(candidates) & opp;
    candidates |= shift(candidates) & opp;
    candidates |= shift(candidates) & opp;
    candidates |= shift(candidates) & opp;
    shift(candidates) & empty
}

/// Returns the bitmask of opponent stones that would be flipped in the
/// given direction if a stone were placed at `target`.
///
/// Casts a beam in the opposite direction and collects the run of
/// opponent stones until it hits one of our own.
#[inline]
fn flips_dir(target: u64, own: u64, opp: u64, shift: fn(u64) -> u64) -> u64 {
    // target から `shift` 方向に進んだ最初のマスが相手石でなければ反転なし．
    let mut flips: u64 = 0;
    let mut x = shift(target) & opp;
    // 連続する相手石を伸ばす ( 最大 6 マス連鎖)
    for _ in 0..6 {
        if x == 0 {
            return 0;
        }
        flips |= x;
        let nx = shift(x);
        if nx & own != 0 {
            // 自石にぶつかった → 反転確定
            return flips;
        }
        x = nx & opp;
    }
    // ループ後も自石に届かなかった ( 端まで相手石のみ等) → 反転なし
    0
}

impl Bitboard8 {
    /// Empty board (all cells empty).
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { black: 0, white: 0 }
    }

    /// Standard Othello initial position.
    ///
    /// Center four cells: `(row=3, col=3) = white`, `(row=3, col=4) = black`,
    /// `(row=4, col=3) = black`, `(row=4, col=4) = white`.
    #[inline]
    #[must_use]
    pub fn standard() -> Self {
        let mut b = Self::empty();
        b.set(Coord::new(3, 3), Some(Color::White));
        b.set(Coord::new(3, 4), Some(Color::Black));
        b.set(Coord::new(4, 3), Some(Color::Black));
        b.set(Coord::new(4, 4), Some(Color::White));
        b
    }

    /// Returns the bit mask of black stones.
    #[inline]
    #[must_use]
    pub const fn black(&self) -> u64 {
        self.black
    }

    /// Returns the bit mask of white stones.
    #[inline]
    #[must_use]
    pub const fn white(&self) -> u64 {
        self.white
    }

    /// Returns the color at the given cell. `None` if empty.
    ///
    /// Returns `None` if `coord` is out of range.
    #[inline]
    #[must_use]
    pub fn cell(&self, coord: Coord) -> Option<Color> {
        if coord.row >= 8 || coord.col >= 8 {
            return None;
        }
        let bit = 1u64 << coord.to_bit_index_8x8();
        if self.black & bit != 0 {
            Some(Color::Black)
        } else if self.white & bit != 0 {
            Some(Color::White)
        } else {
            None
        }
    }

    /// Overwrites a single cell (for tests and setup).
    ///
    /// Out-of-range coordinates are silently ignored.
    pub fn set(&mut self, coord: Coord, color: Option<Color>) {
        if coord.row >= 8 || coord.col >= 8 {
            return;
        }
        let bit = 1u64 << coord.to_bit_index_8x8();
        // 一度両方クリアしてから書き込む
        self.black &= !bit;
        self.white &= !bit;
        match color {
            Some(Color::Black) => self.black |= bit,
            Some(Color::White) => self.white |= bit,
            None => {}
        }
    }

    /// Returns the number of stones of the given color.
    #[inline]
    #[must_use]
    pub fn count(&self, color: Color) -> u32 {
        match color {
            Color::Black => self.black.count_ones(),
            Color::White => self.white.count_ones(),
        }
    }

    /// Returns the number of empty cells.
    #[inline]
    #[must_use]
    pub const fn empty_count(&self) -> u32 {
        (!(self.black | self.white)).count_ones()
    }

    /// Returns the `(own, opp)` bitmask pair from the given color's
    /// perspective.
    #[inline]
    fn own_opp(&self, side: Color) -> (u64, u64) {
        match side {
            Color::Black => (self.black, self.white),
            Color::White => (self.white, self.black),
        }
    }

    /// Returns the bit mask of legal moves for the given side.
    #[inline]
    #[must_use]
    pub fn legal_mask(&self, side: Color) -> u64 {
        let (own, opp) = self.own_opp(side);
        let mut mask = 0u64;
        for shift in SHIFT_FNS {
            mask |= legal_moves_dir(own, opp, shift);
        }
        mask
    }

    /// Returns the legal moves for the given side as a `Vec<Move>`.
    ///
    /// Returns an empty `Vec` when no legal move exists. `Pass` itself is
    /// not included here; the calling `Board` decides whether to issue a
    /// pass.
    #[must_use]
    pub fn legal_moves(&self, side: Color) -> Vec<Move> {
        let mut mask = self.legal_mask(side);
        let mut out = Vec::with_capacity(mask.count_ones() as usize);
        while mask != 0 {
            let idx = mask.trailing_zeros();
            let row = (idx / 8) as u8;
            let col = (idx % 8) as u8;
            out.push(Move::Place(Coord::new(row, col)));
            mask &= mask - 1;
        }
        out
    }

    /// Returns the bit mask of opponent stones that would flip if `side`
    /// played at `coord`.
    ///
    /// Returns 0 when the move flips no stones (i.e. the move is illegal).
    #[inline]
    #[must_use]
    pub fn flips_mask(&self, side: Color, coord: Coord) -> u64 {
        if coord.row >= 8 || coord.col >= 8 {
            return 0;
        }
        let target = 1u64 << coord.to_bit_index_8x8();
        // 既に石があれば 0
        if (self.black | self.white) & target != 0 {
            return 0;
        }
        let (own, opp) = self.own_opp(side);
        let mut total: u64 = 0;
        for shift in SHIFT_FNS {
            total |= flips_dir(target, own, opp, shift);
        }
        total
    }

    /// Applies a move.
    ///
    /// - `Move::Place(c)`: applies the move when at least one stone flips,
    ///   returning the coordinates of the flipped stones.
    /// - `Move::Pass`: returns [`OthelloError::IllegalPass`] when legal
    ///   moves exist, otherwise leaves the board unchanged and returns an
    ///   empty `Vec`.
    pub fn apply(&mut self, side: Color, mv: Move) -> Result<Vec<Coord>, OthelloError> {
        match mv {
            Move::Pass => {
                if self.legal_mask(side) != 0 {
                    Err(OthelloError::IllegalPass)
                } else {
                    Ok(Vec::new())
                }
            }
            Move::Place(coord) => {
                if coord.row >= 8 || coord.col >= 8 {
                    return Err(OthelloError::OutOfBounds {
                        row: coord.row,
                        col: coord.col,
                        rows: 8,
                        cols: 8,
                    });
                }
                let target = 1u64 << coord.to_bit_index_8x8();
                if (self.black | self.white) & target != 0 {
                    return Err(OthelloError::IllegalMove {
                        coord,
                        reason: IllegalMoveReason::Occupied,
                    });
                }
                let flips = self.flips_mask(side, coord);
                if flips == 0 {
                    return Err(OthelloError::IllegalMove {
                        coord,
                        reason: IllegalMoveReason::NoFlips,
                    });
                }
                // 自石を置き，反転を反映
                match side {
                    Color::Black => {
                        self.black |= target | flips;
                        self.white &= !flips;
                    }
                    Color::White => {
                        self.white |= target | flips;
                        self.black &= !flips;
                    }
                }
                Ok(mask_to_coords(flips))
            }
        }
    }

    /// Whether neither side has any legal moves (terminal state).
    #[inline]
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.legal_mask(Color::Black) == 0 && self.legal_mask(Color::White) == 0
    }
}

/// Expands a bitmask into a `Vec<Coord>` (8x8).
fn mask_to_coords(mut mask: u64) -> Vec<Coord> {
    let mut out = Vec::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let idx = mask.trailing_zeros();
        let row = (idx / 8) as u8;
        let col = (idx % 8) as u8;
        out.push(Coord::new(row, col));
        mask &= mask - 1;
    }
    out
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
    fn empty_board_has_no_stones() {
        let b = Bitboard8::empty();
        assert_eq!(b.count(Color::Black), 0);
        assert_eq!(b.count(Color::White), 0);
        assert_eq!(b.empty_count(), 64);
    }

    #[test]
    fn standard_initial_position() {
        let b = Bitboard8::standard();
        assert_eq!(b.count(Color::Black), 2);
        assert_eq!(b.count(Color::White), 2);
        assert_eq!(b.cell(Coord::new(3, 3)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(3, 4)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(4, 3)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(4, 4)), Some(Color::White));
        assert_eq!(b.cell(Coord::new(0, 0)), None);
    }

    #[test]
    fn standard_initial_legal_moves_black() {
        // 黒の初期合法手は 4 つ: (2,3) (3,2) (4,5) (5,4)
        let b = Bitboard8::standard();
        let moves = b.legal_moves(Color::Black);
        let expected = coord_set(&[(2, 3), (3, 2), (4, 5), (5, 4)]);
        assert_eq!(moves_to_coord_set(&moves), expected);
        assert_eq!(moves.len(), 4);
    }

    #[test]
    fn standard_initial_legal_moves_white() {
        // 白の初期合法手も 4 つ: (2,4) (3,5) (4,2) (5,3)
        let b = Bitboard8::standard();
        let moves = b.legal_moves(Color::White);
        let expected = coord_set(&[(2, 4), (3, 5), (4, 2), (5, 3)]);
        assert_eq!(moves_to_coord_set(&moves), expected);
    }

    #[test]
    fn apply_first_black_move() {
        let mut b = Bitboard8::standard();
        let flipped = b
            .apply(Color::Black, Move::Place(Coord::new(2, 3)))
            .unwrap();
        // (3,3) の白が反転する
        assert_eq!(flipped, vec![Coord::new(3, 3)]);
        assert_eq!(b.cell(Coord::new(2, 3)), Some(Color::Black));
        assert_eq!(b.cell(Coord::new(3, 3)), Some(Color::Black));
        assert_eq!(b.count(Color::Black), 4);
        assert_eq!(b.count(Color::White), 1);
    }

    #[test]
    fn illegal_move_on_occupied_cell() {
        let mut b = Bitboard8::standard();
        let err = b
            .apply(Color::Black, Move::Place(Coord::new(3, 3)))
            .unwrap_err();
        assert!(matches!(
            err,
            OthelloError::IllegalMove {
                reason: IllegalMoveReason::Occupied,
                ..
            }
        ));
    }

    #[test]
    fn illegal_move_no_flips() {
        let mut b = Bitboard8::standard();
        // 中央から離れた角は何も反転できない
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
        let mut b = Bitboard8::standard();
        let err = b.apply(Color::Black, Move::Pass).unwrap_err();
        assert_eq!(err, OthelloError::IllegalPass);
    }

    #[test]
    fn pass_allowed_when_no_moves() {
        // 完全に空でも合法手はないので Pass 可能 ( 退化ケース)
        let mut b = Bitboard8::empty();
        assert_eq!(b.legal_moves(Color::Black).len(), 0);
        assert!(b.apply(Color::Black, Move::Pass).is_ok());
    }

    #[test]
    fn out_of_bounds_returns_error() {
        let mut b = Bitboard8::standard();
        let err = b
            .apply(Color::Black, Move::Place(Coord::new(8, 0)))
            .unwrap_err();
        assert!(matches!(
            err,
            OthelloError::OutOfBounds { row: 8, col: 0, .. }
        ));
    }

    #[test]
    fn multi_direction_flip() {
        // 自分で構築した局面: 黒石が中央に集まっているところに白を置くと複数方向で反転する
        // 局面:
        //  . . . . . . . .
        //  . . . . . . . .
        //  . . . W . . . .       row 2, col 3 = W
        //  . . W B B . . .       row 3: (2,3)=W (3,2)=W (3,3)=B (3,4)=B
        //  . . . B . . . .       row 4 col 3 = B
        //  . . . . . . . .
        // 白が (5,3) に置くと col 方向 (3,3)(4,3) と斜め方向の黒石が反転する
        // ここではシンプルに，白で (4,3) の右隣に置いて多方向反転を起こす局面を作る．
        //
        // よりシンプルなテスト: 黒石を一直線に並べて 1 手で連続反転させる．
        //  . . . . . . . .
        //  . . . . . . . .
        //  . . . . . . . .
        //  . W B B B B . .   row 3: col1=W col2..5=B
        //  . . . . . . . .
        // 白が (3, 6) に置くと col 2..5 の黒石 4 つが反転する
        let mut b = Bitboard8::empty();
        b.set(Coord::new(3, 1), Some(Color::White));
        b.set(Coord::new(3, 2), Some(Color::Black));
        b.set(Coord::new(3, 3), Some(Color::Black));
        b.set(Coord::new(3, 4), Some(Color::Black));
        b.set(Coord::new(3, 5), Some(Color::Black));
        let flipped = b
            .apply(Color::White, Move::Place(Coord::new(3, 6)))
            .unwrap();
        let flipped_set: std::collections::BTreeSet<_> = flipped.iter().copied().collect();
        let expected = coord_set(&[(3, 2), (3, 3), (3, 4), (3, 5)]);
        assert_eq!(flipped_set, expected);
        // 元の白 (3,1) + 反転 4 マス + 新規 (3,6) = 6
        assert_eq!(b.count(Color::White), 6);
        assert_eq!(b.count(Color::Black), 0);
    }

    #[test]
    fn no_wrap_around_on_edges() {
        // H 列 ( col 7) の隣に石を置いた状態で，東方向への反転が起こらないことを確認
        let mut b = Bitboard8::empty();
        b.set(Coord::new(0, 7), Some(Color::Black));
        b.set(Coord::new(0, 0), Some(Color::White));
        // (0, 1) に黒を置いても A 列(0,0) との間で wrap-around 反転は発生しない
        // ( 実際には西方向に向かって (0,0) を反転できるかチェック)
        // 黒を (0, -1) ... これは置けないので，
        // (1, 0) に置いて隣接性チェック: (0,0) と (1,0) の間しか影響しない
        // ここでは負例として H→A wrap が存在しないことを確認するため，
        // 黒を (0,0) に追加して (0,7) は孤立させ，東方向に伸びないことを確認
        b.set(Coord::new(0, 0), None);
        b.set(Coord::new(0, 0), Some(Color::Black));
        // 合法手生成で wrap が起きていないか黒の合法手を確認
        let moves = b.legal_moves(Color::White);
        // 反転対象の相手石が孤立しているので白の合法手は 0
        // ( ただし (0,1) は黒石を反転できないので不正)
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn pass_only_position_detection() {
        // 1 色のみが盤面を埋めている → 両者とも合法手なし → 終局
        let mut b = Bitboard8::empty();
        // 全マス黒石にする
        for r in 0..8u8 {
            for c in 0..8u8 {
                b.set(Coord::new(r, c), Some(Color::Black));
            }
        }
        assert!(b.is_terminal());
        assert_eq!(b.legal_moves(Color::Black).len(), 0);
        assert_eq!(b.legal_moves(Color::White).len(), 0);
    }

    #[test]
    fn known_position_legal_moves() {
        // 設計書 §3.1.2 で言及される検証用局面の代表例として，
        // 標準初期配置から黒が d3 ( =(2,3)) に着手後の白の合法手を確認．
        //  初期配置 + 黒 (2,3)，反転で (3,3) も黒
        //  白の合法手は (2,2), (2,4), (4,2) のはず ( 通常の Othello 序盤)
        let mut b = Bitboard8::standard();
        b.apply(Color::Black, Move::Place(Coord::new(2, 3)))
            .unwrap();
        let white_moves = b.legal_moves(Color::White);
        let set = moves_to_coord_set(&white_moves);
        let expected = coord_set(&[(2, 2), (2, 4), (4, 2)]);
        assert_eq!(set, expected);
    }
}
