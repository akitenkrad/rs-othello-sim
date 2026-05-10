//! Legal-move generation and stone-flipping logic for arbitrary-sized boards.
//!
//! [`GenericBoard`](crate::generic_board::GenericBoard) calls the functions
//! in this module to compute legal moves and flips. For each of the eight
//! directions, walk from the placed cell while opponent stones continue,
//! and once a stone of `side` is hit, include all opponent stones along
//! the line in the flip list.

use crate::color::Color;
use crate::coord::Coord;

/// Eight directional offsets (drow, dcol).
pub const DIRECTIONS: [(i8, i8); 8] = [
    (-1, 0),  // N
    (1, 0),   // S
    (0, 1),   // E
    (0, -1),  // W
    (-1, 1),  // NE
    (-1, -1), // NW
    (1, 1),   // SE
    (1, -1),  // SW
];

/// Returns whether `(row, col)` is inside the board.
#[inline]
pub fn in_bounds(row: i32, col: i32, rows: u8, cols: u8) -> bool {
    row >= 0 && row < rows as i32 && col >= 0 && col < cols as i32
}

/// Returns every opponent-stone coordinate that would flip if `side` plays
/// at `target`.
///
/// `cell_at` is a closure that takes `(row, col)` and returns
/// `Option<Color>`. The return value collects all flipped opponent stones
/// across the eight directions.
///
/// Returns an empty `Vec` when the move flips no stones (i.e. it is
/// illegal).
pub fn flips_for_move<F>(side: Color, target: Coord, rows: u8, cols: u8, cell_at: F) -> Vec<Coord>
where
    F: Fn(u8, u8) -> Option<Color>,
{
    if target.row >= rows || target.col >= cols {
        return Vec::new();
    }
    if cell_at(target.row, target.col).is_some() {
        return Vec::new();
    }

    let opp = side.opponent();
    let mut all_flips: Vec<Coord> = Vec::new();

    for (dr, dc) in DIRECTIONS {
        let mut r = target.row as i32 + dr as i32;
        let mut c = target.col as i32 + dc as i32;
        let mut line: Vec<Coord> = Vec::new();
        // 連続する相手石を集める
        while in_bounds(r, c, rows, cols) {
            match cell_at(r as u8, c as u8) {
                Some(color) if color == opp => {
                    line.push(Coord::new(r as u8, c as u8));
                    r += dr as i32;
                    c += dc as i32;
                }
                Some(color) if color == side => {
                    // 自石にぶつかった → ここまでの相手石が反転対象
                    if !line.is_empty() {
                        all_flips.extend(line.iter().copied());
                    }
                    break;
                }
                _ => break, // 空マスや盤外 → この方向は反転なし
            }
        }
    }

    all_flips
}

/// Enumerates all legal-move coordinates for the given side.
pub fn legal_move_coords<F>(side: Color, rows: u8, cols: u8, cell_at: F) -> Vec<Coord>
where
    F: Fn(u8, u8) -> Option<Color>,
{
    let mut out = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            if cell_at(r, c).is_some() {
                continue;
            }
            // 1 方向でも反転があれば合法手
            if has_any_flip(side, Coord::new(r, c), rows, cols, &cell_at) {
                out.push(Coord::new(r, c));
            }
        }
    }
    out
}

/// Returns `true` as soon as at least one flip is found (fast early return).
fn has_any_flip<F>(side: Color, target: Coord, rows: u8, cols: u8, cell_at: &F) -> bool
where
    F: Fn(u8, u8) -> Option<Color>,
{
    let opp = side.opponent();
    for (dr, dc) in DIRECTIONS {
        let mut r = target.row as i32 + dr as i32;
        let mut c = target.col as i32 + dc as i32;
        let mut found_opp = false;
        while in_bounds(r, c, rows, cols) {
            match cell_at(r as u8, c as u8) {
                Some(color) if color == opp => {
                    found_opp = true;
                    r += dr as i32;
                    c += dc as i32;
                }
                Some(color) if color == side => {
                    if found_opp {
                        return true;
                    }
                    break;
                }
                _ => break,
            }
        }
    }
    false
}
