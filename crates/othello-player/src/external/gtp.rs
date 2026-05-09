//! GTP ( Go Text Protocol) 風プロトコルの実装．
//!
//! 設計書 §3.2.2 の GTP 風通信を Othello 用に簡略化している．
//! コマンドは 1 行ごとに送り，応答は `= ...\n\n` ( 成功) または `? ...\n\n` ( 失敗)．

use super::protocol::{EngineProtocol, coord_to_gtp, parse_gtp_coord};
use crate::traits::PlayerError;
use othello_core::{BoardSize, Color, GameState, Move};
use std::io::{BufRead, Write};
use std::time::Duration;

/// GTP プロトコル状態．
#[derive(Debug, Default)]
pub struct GtpProtocol;

impl GtpProtocol {
    /// 新規 GTP プロトコルインスタンス．
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl EngineProtocol for GtpProtocol {
    fn start_game(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        board_size: BoardSize,
        timeout: Duration,
    ) -> Result<(), PlayerError> {
        send_command(stdin, &format!("boardsize {}", board_size.rows))?;
        read_gtp_response(stdout, timeout)?;
        send_command(stdin, "clear_board")?;
        read_gtp_response(stdout, timeout)?;
        Ok(())
    }

    fn notify_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        mv: Move,
        timeout: Duration,
    ) -> Result<(), PlayerError> {
        let color = color_str(side);
        let coord = match mv {
            Move::Place(c) => coord_to_gtp(c.row, c.col),
            Move::Pass => "pass".to_string(),
        };
        send_command(stdin, &format!("play {color} {coord}"))?;
        read_gtp_response(stdout, timeout)?;
        Ok(())
    }

    fn request_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        _state: &GameState,
        timeout: Duration,
    ) -> Result<Move, PlayerError> {
        send_command(stdin, &format!("genmove {}", color_str(side)))?;
        let body = read_gtp_response(stdout, timeout)?;
        match parse_gtp_coord(&body)? {
            None => Ok(Move::Pass),
            Some((row, col)) => Ok(Move::Place(othello_core::Coord::new(row, col))),
        }
    }

    fn quit(&mut self, stdin: &mut dyn Write) -> Result<(), PlayerError> {
        // quit は best-effort ( 応答待ちはしない)．
        let _ = writeln!(stdin, "quit");
        let _ = stdin.flush();
        Ok(())
    }
}

fn color_str(side: Color) -> &'static str {
    match side {
        Color::Black => "black",
        Color::White => "white",
    }
}

fn send_command(stdin: &mut dyn Write, line: &str) -> Result<(), PlayerError> {
    writeln!(stdin, "{line}").map_err(PlayerError::from)?;
    stdin.flush().map_err(PlayerError::from)?;
    Ok(())
}

/// GTP 応答を読む ( `= body\n\n` 形式)．戻り値は body 部分．
///
/// `?` で始まる行はエラーとして `PlayerError::Other` で返す．空行を区切りに使う．
/// 簡易タイムアウトは [`super::process::read_lines_with_timeout`] 相当を呼び出し側で行う想定．
/// ここでは BufRead の `read_line` を素直に使い，呼び出し側のスレッド分離タイムアウトに任せる．
fn read_gtp_response(stdout: &mut dyn BufRead, _timeout: Duration) -> Result<String, PlayerError> {
    // 空行までの全行を集める．先頭行が `=` または `?`．
    let mut first: Option<String> = None;
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
        if trimmed.is_empty() {
            // 空行 = 応答終了
            break;
        }
        if first.is_none() {
            first = Some(trimmed);
        }
        // 後続行は無視 ( 単一手の応答想定)
    }
    let head = first.ok_or_else(|| PlayerError::Other("empty GTP response".into()))?;
    if let Some(rest) = head.strip_prefix("=") {
        Ok(rest.trim().to_string())
    } else if let Some(rest) = head.strip_prefix("?") {
        Err(PlayerError::Other(format!("engine error: {}", rest.trim())))
    } else {
        Err(PlayerError::Other(format!(
            "unexpected GTP response: {head:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_response_success() {
        let mut cur = Cursor::new(b"= D3\n\n".to_vec());
        let body = read_gtp_response(&mut cur, Duration::from_secs(1)).unwrap();
        assert_eq!(body, "D3");
    }

    #[test]
    fn parse_response_pass() {
        let mut cur = Cursor::new(b"= pass\n\n".to_vec());
        let body = read_gtp_response(&mut cur, Duration::from_secs(1)).unwrap();
        assert_eq!(body, "pass");
    }

    #[test]
    fn parse_response_empty_ok() {
        // boardsize 等は空 body
        let mut cur = Cursor::new(b"= \n\n".to_vec());
        let body = read_gtp_response(&mut cur, Duration::from_secs(1)).unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn parse_response_error() {
        let mut cur = Cursor::new(b"? unknown command\n\n".to_vec());
        let r = read_gtp_response(&mut cur, Duration::from_secs(1));
        assert!(r.is_err());
    }
}
