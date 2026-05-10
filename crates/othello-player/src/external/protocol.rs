//! Abstraction over external engine communication.
//!
//! Design doc §3.2.2 calls for "GTP-like" communication; for compatibility
//! with real Edax / Egaroucid binaries we switch between two dialects via
//! the [`Protocol`] enum (see [`super::gtp`], [`super::ntest`]).
//!
//! Communication is performed over synchronous IO
//! (`BufReader<ChildStdout>` / `ChildStdin`).

use crate::traits::PlayerError;
use othello_core::{BoardSize, Color, GameState, Move};
use std::io::{BufRead, Write};
use std::time::Duration;

/// Communication dialect for an external engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Go Text Protocol-like (the default).
    ///
    /// Main commands:
    /// - `boardsize <N>`
    /// - `clear_board`
    /// - `play <color> <coord>` (`play black D3`, `play white pass`)
    /// - `genmove <color>` -> response `= D3` / `= pass`
    /// - `quit`
    Gtp,
    /// Simplified protocol leaning toward Edax / Egaroucid.
    ///
    /// Main commands:
    /// - `set game <board_str>` to send the position as a board string
    /// - `go` -> response `D3` / `pa`
    Ntest,
}

impl Protocol {
    /// Parses `Protocol` from a string (case-insensitive).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "gtp" => Ok(Self::Gtp),
            "ntest" | "edax" | "egaroucid" => Ok(Self::Ntest),
            other => Err(format!("unknown protocol: {other:?}")),
        }
    }

    /// Returns the lowercase display name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gtp => "gtp",
            Self::Ntest => "ntest",
        }
    }
}

/// Trait abstracting one session (one game) with an external engine.
///
/// Implementations are required to **reuse the same process** so that
/// `reset` / `play` / `genmove` can be called in sequence.
pub trait EngineProtocol {
    /// Starts a new game (sends `clear_board` etc.).
    fn start_game(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        board_size: BoardSize,
        timeout: Duration,
    ) -> Result<(), PlayerError>;

    /// Notifies the engine of a move (used for both our moves and the
    /// opponent's).
    fn notify_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        mv: Move,
        timeout: Duration,
    ) -> Result<(), PlayerError>;

    /// Requests the engine to think and returns the chosen move.
    fn request_move(
        &mut self,
        stdin: &mut dyn Write,
        stdout: &mut dyn BufRead,
        side: Color,
        state: &GameState,
        timeout: Duration,
    ) -> Result<Move, PlayerError>;

    /// Notifies the engine of process termination (sends `quit` etc.).
    fn quit(&mut self, stdin: &mut dyn Write) -> Result<(), PlayerError>;
}

/// Builds a GTP coordinate string (e.g. `D3`) from `(row, col)`.
#[must_use]
pub fn coord_to_gtp(row: u8, col: u8) -> String {
    let col_char = (b'A' + col) as char;
    format!("{col_char}{}", row + 1)
}

/// Parses a GTP coordinate string (case-insensitive) into 0-indexed
/// `(row, col)`.
///
/// `pass` / `PASS` are also accepted and return `None`.
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
