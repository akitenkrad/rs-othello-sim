//! ntest (Edax / Egaroucid-leaning) simplified protocol.
//!
//! - `set game <board_str>` sends the position as a board string
//!   (WTHOR-style).
//! - `go` requests a move; the response is `<coord>` (e.g. `D3`) or `pa`
//!   (pass).
//!
//! The protocol is simplified so that every response fits in one line.
//! It does not match real Edax behaviour exactly, but it is sufficient
//! for testing mock scripts and validating the protocol-layer
//! abstraction.

use super::protocol::{EngineProtocol, parse_gtp_coord};
use crate::traits::PlayerError;
use othello_core::{BoardSize, Color, Coord, GameState, Move};
use std::io::{BufRead, Write};
use std::time::Duration;

/// State of the ntest protocol.
#[derive(Debug, Default)]
pub struct NtestProtocol;

impl NtestProtocol {
    /// Creates a new `NtestProtocol` instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl EngineProtocol for NtestProtocol {
    fn start_game(
        &mut self,
        _stdin: &mut dyn Write,
        _stdout: &mut dyn BufRead,
        _board_size: BoardSize,
        _timeout: Duration,
    ) -> Result<(), PlayerError> {
        // ntest 系は `set game` で局面を都度送るので，開始処理は不要．
        Ok(())
    }

    fn notify_move(
        &mut self,
        _stdin: &mut dyn Write,
        _stdout: &mut dyn BufRead,
        _side: Color,
        _mv: Move,
        _timeout: Duration,
    ) -> Result<(), PlayerError> {
        // notify は不要 ( request_move のたびに局面全体を送る)．
        Ok(())
    }

    fn request_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        state: &GameState,
        _timeout: Duration,
    ) -> Result<Move, PlayerError> {
        let board_str = encode_board(state, side);
        send_line(stdin, &format!("set game {board_str}"))?;
        send_line(stdin, "go")?;
        let line = read_one_line(stdout)?;
        match parse_gtp_coord(&line)? {
            None => Ok(Move::Pass),
            Some((row, col)) => Ok(Move::Place(Coord::new(row, col))),
        }
    }

    fn quit(&mut self, stdin: &mut dyn Write) -> Result<(), PlayerError> {
        let _ = writeln!(stdin, "quit");
        let _ = stdin.flush();
        Ok(())
    }
}

/// Encodes a position as an ntest-style board string.
///
/// Format: 64 characters (assuming 8x8) followed by ` <X|O>` indicating
/// the side to move. `*` = black stone, `O` = white stone, `-` = empty.
/// Example: `---------------------------O*------*O--------------------------- *`
///
/// For non-8x8 boards the string length depends on the size.
fn encode_board(state: &GameState, side: Color) -> String {
    let size = state.board.size();
    let mut s = String::with_capacity((size.rows as usize) * (size.cols as usize) + 2);
    for r in 0..size.rows {
        for c in 0..size.cols {
            let ch = match state.board.cell(Coord::new(r, c)) {
                Some(Color::Black) => '*',
                Some(Color::White) => 'O',
                None => '-',
            };
            s.push(ch);
        }
    }
    s.push(' ');
    s.push(match side {
        Color::Black => '*',
        Color::White => 'O',
    });
    s
}

fn send_line(stdin: &mut dyn Write, line: &str) -> Result<(), PlayerError> {
    writeln!(stdin, "{line}").map_err(PlayerError::from)?;
    stdin.flush().map_err(PlayerError::from)?;
    Ok(())
}

fn read_one_line(stdout: &mut dyn BufRead) -> Result<String, PlayerError> {
    loop {
        let mut line = String::new();
        let n = stdout
            .read_line(&mut line)
            .map_err(|e| PlayerError::Other(format!("read failed: {e}")))?;
        if n == 0 {
            return Err(PlayerError::Other(
                "engine closed stdout unexpectedly".into(),
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
        // 空行はスキップ
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::GameState;

    #[test]
    fn encode_initial_8x8() {
        let s = GameState::standard_8x8();
        let board = encode_board(&s, Color::Black);
        // 末尾 ` *` を除いた 64 文字．
        assert_eq!(board.len(), 64 + 2);
        assert!(board.ends_with(" *"));
        // 中央の D4=(3,3) と E5=(4,4) は白，D5=(3,4) と E4=(4,3) は黒．
        // index = row * 8 + col．board[27]=D4=O, board[28]=E4=*, board[35]=D5=*, board[36]=E5=O
        assert_eq!(board.as_bytes()[27], b'O');
        assert_eq!(board.as_bytes()[28], b'*');
        assert_eq!(board.as_bytes()[35], b'*');
        assert_eq!(board.as_bytes()[36], b'O');
    }
}
