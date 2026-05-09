//! [`HumanPlayer`]: 標準入力 ( または任意の `Read`) から座標文字列を読み取るプレイヤー．
//!
//! 入力フォーマット:
//! - 座標: 小文字英字 + 数字 ( 例 `e4`)．列 a-z ( 1-26)，行 1-26．
//! - パス: `pass` または `Pass` ( 大文字小文字無視)．
//! - 不正入力時はテスト容易性のため再入力を促さず `PlayerError::InvalidCoord` を返す．

use crate::traits::{Player, PlayerError};
use othello_core::{Color, Coord, GameState, Move};
use std::io::{BufRead, BufReader, Read, Write};

/// 標準入力 ( または任意の `Read`) からの対話入力で手を選ぶプレイヤー．
///
/// テスト容易性のため `Read` トレイトオブジェクトと `Write` トレイトオブジェクト ( プロンプト出力用) を
/// 注入可能にしている．デフォルトでは `stdin` / `stderr` を使う．
pub struct HumanPlayer {
    name: String,
    color: Color,
    reader: Box<dyn BufRead + Send>,
    writer: Box<dyn Write + Send>,
}

impl HumanPlayer {
    /// 標準入力からの読み取り + 標準エラー出力へのプロンプトでインスタンス化する．
    #[must_use]
    pub fn new_stdio(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            color,
            reader: Box::new(BufReader::new(std::io::stdin())),
            writer: Box::new(std::io::stderr()),
        }
    }

    /// 任意の `Read` と `Write` を注入してインスタンス化する ( テスト用)．
    pub fn new_with_io<R, W>(name: impl Into<String>, color: Color, reader: R, writer: W) -> Self
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        Self {
            name: name.into(),
            color,
            reader: Box::new(reader),
            writer: Box::new(writer),
        }
    }

    /// 任意の `Read` ( `BufRead` でない) を渡したい場合のヘルパ．
    pub fn new_from_read<R, W>(name: impl Into<String>, color: Color, reader: R, writer: W) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        Self::new_with_io(name, color, BufReader::new(reader), writer)
    }
}

/// `e4` のような座標文字列をパースする．
///
/// - 列: 小文字英字 1 文字 ( a-z) → 0-indexed 列番号
/// - 行: 1〜2 桁の正整数 ( 1-indexed) → 0-indexed 行番号
///
/// 大文字英字も受理する ( `E4` も OK)．
pub fn parse_coord(input: &str) -> Result<Coord, PlayerError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PlayerError::InvalidCoord {
            input: input.to_string(),
            reason: "empty input",
        });
    }

    let mut chars = trimmed.chars();
    let col_char = chars.next().ok_or(PlayerError::InvalidCoord {
        input: input.to_string(),
        reason: "missing column letter",
    })?;
    if !col_char.is_ascii_alphabetic() {
        return Err(PlayerError::InvalidCoord {
            input: input.to_string(),
            reason: "first character must be a letter (a-z)",
        });
    }
    let col = (col_char.to_ascii_lowercase() as u32 - 'a' as u32) as u8;
    if col >= 26 {
        return Err(PlayerError::InvalidCoord {
            input: input.to_string(),
            reason: "column must be a-z",
        });
    }

    let rest: String = chars.collect();
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return Err(PlayerError::InvalidCoord {
            input: input.to_string(),
            reason: "row must be a positive integer",
        });
    }
    let row_1: u32 = rest.parse().map_err(|_| PlayerError::InvalidCoord {
        input: input.to_string(),
        reason: "row must be a positive integer",
    })?;
    if !(1..=26).contains(&row_1) {
        return Err(PlayerError::InvalidCoord {
            input: input.to_string(),
            reason: "row must be in 1..=26",
        });
    }
    let row = (row_1 - 1) as u8;
    Ok(Coord::new(row, col))
}

impl Player for HumanPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color {
        self.color
    }

    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError> {
        // プロンプト
        let legal = state.legal_moves();
        let prompt = format!(
            "{:?} to move (legal: {}). Enter move (e.g. d3, pass): ",
            state.side_to_move,
            format_legal_moves(&legal)
        );
        let _ = self.writer.write_all(prompt.as_bytes());
        let _ = self.writer.flush();

        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(PlayerError::InputExhausted);
        }
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("pass") {
            return Ok(Move::Pass);
        }

        let coord = parse_coord(trimmed)?;
        // 範囲チェック ( 盤外も InvalidCoord として返す)
        let size = state.board.size();
        if coord.row >= size.rows || coord.col >= size.cols {
            return Err(PlayerError::InvalidCoord {
                input: trimmed.to_string(),
                reason: "coordinate out of bounds",
            });
        }
        Ok(Move::Place(coord))
    }
}

fn format_legal_moves(moves: &[Move]) -> String {
    let mut parts = Vec::new();
    for m in moves {
        match m {
            Move::Place(c) => parts.push(format_coord(*c)),
            Move::Pass => parts.push("pass".to_string()),
        }
    }
    if parts.is_empty() {
        "pass".to_string()
    } else {
        parts.join(", ")
    }
}

/// `Coord` を `"e4"` 形式の文字列に変換する．
#[must_use]
pub fn format_coord(c: Coord) -> String {
    let col_char = (b'a' + c.col) as char;
    format!("{}{}", col_char, c.row + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_coord() {
        let c = parse_coord("d3").unwrap();
        assert_eq!(c, Coord::new(2, 3));
    }

    #[test]
    fn parses_uppercase_coord() {
        let c = parse_coord("E4").unwrap();
        assert_eq!(c, Coord::new(3, 4));
    }

    #[test]
    fn rejects_invalid_coord() {
        assert!(parse_coord("").is_err());
        assert!(parse_coord("3").is_err());
        assert!(parse_coord("d").is_err());
        assert!(parse_coord("dd").is_err());
        assert!(parse_coord("d0").is_err());
        assert!(parse_coord("d27").is_err());
    }

    #[test]
    fn human_player_reads_coord() {
        let input = "d3\n";
        let output: Vec<u8> = Vec::new();
        let mut p = HumanPlayer::new_with_io(
            "Human",
            Color::Black,
            BufReader::new(input.as_bytes()),
            output,
        );
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        assert_eq!(mv, Move::Place(Coord::new(2, 3)));
    }

    #[test]
    fn human_player_reads_pass() {
        let input = "pass\n";
        let output: Vec<u8> = Vec::new();
        let mut p = HumanPlayer::new_with_io(
            "Human",
            Color::Black,
            BufReader::new(input.as_bytes()),
            output,
        );
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        assert_eq!(mv, Move::Pass);
    }

    #[test]
    fn human_player_rejects_garbage() {
        let input = "xyzzy\n";
        let output: Vec<u8> = Vec::new();
        let mut p = HumanPlayer::new_with_io(
            "Human",
            Color::Black,
            BufReader::new(input.as_bytes()),
            output,
        );
        let s = GameState::standard_8x8();
        let result = p.select_move(&s);
        assert!(matches!(result, Err(PlayerError::InvalidCoord { .. })));
    }

    #[test]
    fn format_coord_round_trip() {
        let c = Coord::new(2, 3);
        let s = format_coord(c);
        assert_eq!(s, "d3");
        assert_eq!(parse_coord(&s).unwrap(), c);
    }
}
