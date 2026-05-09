//! ntest ( Edax / Egaroucid 寄り) 簡易プロトコル．
//!
//! - `set game <board_str>` で局面を盤面文字列で送る ( WTHOR 風)
//! - `go` で着手要求 → 応答 `<coord>` ( 例 `D3`) または `pa` ( pass)
//!
//! 応答は 1 行で完結する単純化したプロトコル．実機 Edax の挙動と完全には合わないが，
//! テスト用モックスクリプトとプロトコル層の分離検証目的としては十分．

use super::protocol::{EngineProtocol, parse_gtp_coord};
use crate::traits::PlayerError;
use othello_core::{BoardSize, Color, Coord, GameState, Move};
use std::io::{BufRead, Write};
use std::time::Duration;

/// ntest プロトコル状態．
#[derive(Debug, Default)]
pub struct NtestProtocol;

impl NtestProtocol {
    /// 新規 ntest プロトコルインスタンス．
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

/// 局面を ntest 風の盤面文字列に変換する．
///
/// フォーマット: 64 文字 ( 8x8 想定) + ` <X|O>` ( 手番) ．
/// `*` = 黒石，`O` = 白石，`-` = 空マス．例: `---------------------------O*------*O--------------------------- *`
///
/// 8x8 以外は文字列の長さがサイズ依存になる．
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
