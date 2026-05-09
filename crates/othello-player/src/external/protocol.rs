//! 外部エンジン通信の抽象．
//!
//! 設計書 §3.2.2 では「GTP 風通信」とあるが，実機 Edax / Egaroucid 互換性を考慮し
//! 2 種類の方言を [`Protocol`] enum で切り替える ( see [`super::gtp`], [`super::ntest`])．
//!
//! 通信は同期 IO ( `BufReader<ChildStdout>` / `ChildStdin`) で行う．

use crate::traits::PlayerError;
use othello_core::{BoardSize, Color, GameState, Move};
use std::io::{BufRead, Write};
use std::time::Duration;

/// 外部エンジンとの通信方言．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Go Text Protocol 風 ( デフォルト)．
    ///
    /// 主要コマンド:
    /// - `boardsize <N>`
    /// - `clear_board`
    /// - `play <color> <coord>` ( `play black D3`，`play white pass`)
    /// - `genmove <color>` → 応答 `= D3` / `= pass`
    /// - `quit`
    Gtp,
    /// Edax / Egaroucid 寄りの簡易プロトコル．
    ///
    /// 主要コマンド:
    /// - `set game <board_str>` で局面を盤面文字列で送る
    /// - `go` → 応答 `D3` / `pa`
    Ntest,
}

impl Protocol {
    /// 文字列 ( 大文字小文字無視) から `Protocol` を解析する．
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "gtp" => Ok(Self::Gtp),
            "ntest" | "edax" | "egaroucid" => Ok(Self::Ntest),
            other => Err(format!("unknown protocol: {other:?}")),
        }
    }

    /// 表示用の小文字名．
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gtp => "gtp",
            Self::Ntest => "ntest",
        }
    }
}

/// 外部エンジンとの 1 セッション ( ゲーム単位) を抽象化する trait．
///
/// 各実装は **同一プロセスを使い回し**，`reset` / `play` / `genmove` を順に呼べる構造とする．
pub trait EngineProtocol {
    /// 新規ゲーム開始 ( `clear_board` 等を送出)．
    fn start_game(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        board_size: BoardSize,
        timeout: Duration,
    ) -> Result<(), PlayerError>;

    /// 着手を相手に通知する ( 自手・相手手の両方)．
    fn notify_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        mv: Move,
        timeout: Duration,
    ) -> Result<(), PlayerError>;

    /// 思考を要求し，着手を取得する．
    fn request_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        state: &GameState,
        timeout: Duration,
    ) -> Result<Move, PlayerError>;

    /// プロセス終了を通知する ( `quit` 等)．
    fn quit(&mut self, stdin: &mut dyn Write) -> Result<(), PlayerError>;
}

/// `(row, col)` から GTP 座標文字列 ( 例 `D3`) を生成する．
#[must_use]
pub fn coord_to_gtp(row: u8, col: u8) -> String {
    let col_char = (b'A' + col) as char;
    format!("{col_char}{}", row + 1)
}

/// GTP 座標文字列 ( 大文字小文字無視) を `(row, col)` 0-indexed に解析する．
///
/// `pass` / `PASS` も許容し `None` を返す．
pub fn parse_gtp_coord(s: &str) -> Result<Option<(u8, u8)>, PlayerError> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("pass") || s.eq_ignore_ascii_case("pa") {
        return Ok(None);
    }
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return Err(PlayerError::Other(format!(
            "invalid coord from engine: {s:?}"
        )));
    }
    let col_byte = bytes[0].to_ascii_uppercase();
    if !col_byte.is_ascii_uppercase() {
        return Err(PlayerError::Other(format!(
            "invalid column char in coord: {s:?}"
        )));
    }
    // GTP では 'I' をスキップする実装もあるが，Othello では 8 までなので影響なし．
    let col = col_byte - b'A';
    let row_str = std::str::from_utf8(&bytes[1..])
        .map_err(|e| PlayerError::Other(format!("non-utf8 row in coord {s:?}: {e}")))?;
    let row_1 = row_str
        .parse::<u8>()
        .map_err(|e| PlayerError::Other(format!("invalid row in coord {s:?}: {e}")))?;
    if row_1 == 0 {
        return Err(PlayerError::Other(format!("row must be >= 1: {s:?}")));
    }
    Ok(Some((row_1 - 1, col)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_roundtrip() {
        assert_eq!(coord_to_gtp(0, 0), "A1");
        assert_eq!(coord_to_gtp(2, 3), "D3");
        assert_eq!(coord_to_gtp(7, 7), "H8");
    }

    #[test]
    fn parse_coord_basic() {
        assert_eq!(parse_gtp_coord("D3").unwrap(), Some((2, 3)));
        assert_eq!(parse_gtp_coord("d3").unwrap(), Some((2, 3)));
        assert_eq!(parse_gtp_coord(" A1 ").unwrap(), Some((0, 0)));
        assert_eq!(parse_gtp_coord("pass").unwrap(), None);
        assert_eq!(parse_gtp_coord("PASS").unwrap(), None);
    }

    #[test]
    fn parse_coord_errors() {
        assert!(parse_gtp_coord("").is_err());
        assert!(parse_gtp_coord("3").is_err());
        assert!(parse_gtp_coord("D").is_err());
        assert!(parse_gtp_coord("D0").is_err());
    }

    #[test]
    fn protocol_parse() {
        assert_eq!(Protocol::parse("gtp").unwrap(), Protocol::Gtp);
        assert_eq!(Protocol::parse("Gtp").unwrap(), Protocol::Gtp);
        assert_eq!(Protocol::parse("ntest").unwrap(), Protocol::Ntest);
        assert_eq!(Protocol::parse("edax").unwrap(), Protocol::Ntest);
        assert!(Protocol::parse("unknown").is_err());
    }
}
