//! Subcommands for `othello-cli`.

pub mod benchmark;
pub mod convert;
pub mod fetch;
pub mod inspect;
pub mod observe;
pub mod play;
pub mod replay;
pub mod selfplay;
pub mod simulate;

use othello_core::{Color, Coord, GameState, Move};

/// Format the board and side-to-move information as an ASCII string.
pub fn render_board(state: &GameState) -> String {
    let size = state.board.size();
    let mut s = String::new();
    // 列ヘッダ
    s.push_str("   ");
    for c in 0..size.cols {
        s.push((b'a' + c) as char);
        s.push(' ');
    }
    s.push('\n');
    for r in 0..size.rows {
        s.push_str(&format!("{:>2} ", r + 1));
        for c in 0..size.cols {
            let glyph = match state.board.cell(Coord::new(r, c)) {
                Some(Color::Black) => 'X',
                Some(Color::White) => 'O',
                None => '.',
            };
            s.push(glyph);
            s.push(' ');
        }
        s.push('\n');
    }
    s
}

/// Convert a `Coord` into a string like `"e4"`.
pub fn format_coord(c: Coord) -> String {
    let col_char = (b'a' + c.col) as char;
    format!("{}{}", col_char, c.row + 1)
}

/// Display string for a `Move`.
pub fn format_move(mv: Move) -> String {
    match mv {
        Move::Place(c) => format_coord(c),
        Move::Pass => "pass".to_string(),
    }
}
