//! 任意サイズ盤面用の合法手生成・石返しロジック．
//!
//! [`GenericBoard`](crate::generic_board::GenericBoard) はこのモジュールの関数を呼び出して
//! 合法手・石返しを計算する．8 方向それぞれについて，置いたマスから方向に向かって
//! 相手石が連続している間進み，自石にぶつかったら間の石をすべて反転リストに含める．

use crate::color::Color;
use crate::coord::Coord;

/// 8 方向のオフセット ( drow, dcol)．
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

/// `( row, col)` が盤面内かどうか確認するヘルパ．
#[inline]
pub fn in_bounds(row: i32, col: i32, rows: u8, cols: u8) -> bool {
    row >= 0 && row < rows as i32 && col >= 0 && col < cols as i32
}

/// 指定座標に `side` を置いた場合に反転する相手石の座標を全て返す．
///
/// `cell_at` は `(row, col)` を受け取って `Option<Color>` を返すクロージャ．
/// 返り値は全 8 方向で反転する相手石の座標のリスト．
///
/// 反転 0 マスなら空 `Vec` を返す ( = 不正手)．
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

/// 指定色の合法手座標を全て列挙する．
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

/// 反転が 1 つでもあれば `true` ( 早期 return で高速化)．
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
