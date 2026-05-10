//! Reader/Writer for the GGF (Generic Game Format) Othello subset.
//!
//! Minimal implementation conforming to §4.3 of the design document.
//! Supported tags:
//! - `GM` (game kind) — rejects values other than `Othello`.
//! - `PC` (place / source).
//! - `DT` (date).
//! - `PB` / `PW` (black/white player name).
//! - `RE` (result, score margin).
//! - `BO` (initial board).
//! - `B[...]` / `W[...]` (moves).
//!
//! Reads moves in either form: `B[D3]` or `B[D3//1.234]` (with think
//! time). Writes use the no-time form `B[D3]`. Pass is read as either
//! `B[--]` or `B[PA]` and written as `B[--]`.
//!
//! GGF coordinates are columns A-H (1-indexed) plus rows 1-8. The
//! internal `Coord(row, col)` is 0-indexed and is converted accordingly.

use crate::error::IoError;
use crate::record::{
    GameMetadata, GameRecord, GameResultRecord, MoveEntry, PlayerInfo, PlayerPair, SCHEMA_VERSION,
    Score,
};
use crate::traits::{GameRecordReader, GameRecordWriter};
use chrono::{FixedOffset, Utc};
use othello_core::{BoardSize, Color, Coord, Move};
use std::io::{Read, Write};

/// GGF reader.
#[derive(Debug, Default)]
pub struct GgfReader;

impl GgfReader {
    /// Constructs a new reader.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// GGF writer.
#[derive(Debug, Default)]
pub struct GgfWriter;

impl GgfWriter {
    /// Constructs a new writer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ---- GGF 解析 ----------------------------------------------------------------

/// One GGF tag (a pair such as `GM[Othello]`).
#[derive(Debug)]
struct GgfTag {
    name: String,
    value: String,
}

/// Minimal GGF parser.
///
/// Inputs look like `(;GM[Othello]PB[X]PW[Y]...B[D3]W[C5];)` — a sequence
/// of tag-name + `[` + value + `]`. `(`, `)`, and `;` are structural
/// separators.
fn parse_tags(input: &str) -> Result<Vec<GgfTag>, IoError> {
    let mut tags = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // skip whitespace and structural chars
        while i < bytes.len() {
            let c = bytes[i];
            if c == b'(' || c == b')' || c == b';' || c.is_ascii_whitespace() {
                i += 1;
            } else {
                break;
            }
        }
        if i >= bytes.len() {
            break;
        }
        // Parse tag name (uppercase letters)
        let name_start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i == name_start {
            return Err(IoError::Parse(format!(
                "expected tag name at byte {name_start}"
            )));
        }
        let name = std::str::from_utf8(&bytes[name_start..i])
            .map_err(|_| IoError::Parse("tag name not UTF-8".into()))?
            .to_string();
        // Expect '['
        if i >= bytes.len() || bytes[i] != b'[' {
            return Err(IoError::Parse(format!(
                "expected '[' after tag {name} at byte {i}"
            )));
        }
        i += 1;
        // Read until ']'
        let val_start = i;
        while i < bytes.len() && bytes[i] != b']' {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(IoError::Parse(format!(
                "unclosed tag value for {name} starting at byte {val_start}"
            )));
        }
        let value = std::str::from_utf8(&bytes[val_start..i])
            .map_err(|_| IoError::Parse("tag value not UTF-8".into()))?
            .to_string();
        i += 1; // consume ']'
        tags.push(GgfTag { name, value });
    }

    Ok(tags)
}

/// Converts `D3` or `D3//1.234` into `Move::Place`.
/// `--` or `PA` (case-insensitive) becomes `Move::Pass`.
fn parse_ggf_move(value: &str) -> Result<Move, IoError> {
    // 思考時間サフィックスの除去
    let core = value.split("//").next().unwrap_or(value).trim();
    if core.eq_ignore_ascii_case("pa") || core == "--" {
        return Ok(Move::Pass);
    }
    if core.len() < 2 {
        return Err(IoError::Parse(format!("invalid GGF move: {value:?}")));
    }
    let mut chars = core.chars();
    let col_char = chars.next().unwrap();
    let rest: String = chars.collect();
    if !col_char.is_ascii_alphabetic() {
        return Err(IoError::Parse(format!("invalid GGF column: {col_char:?}")));
    }
    let col = (col_char.to_ascii_uppercase() as u32 - 'A' as u32) as u8;
    let row_1: u8 = rest
        .parse()
        .map_err(|_| IoError::Parse(format!("invalid GGF row: {rest:?}")))?;
    if !(1..=26).contains(&row_1) {
        return Err(IoError::Parse(format!("invalid GGF row: {row_1}")));
    }
    Ok(Move::Place(Coord::new(row_1 - 1, col)))
}

/// Converts a `Move` into a GGF string. Used on the write side.
fn move_to_ggf(mv: Move) -> String {
    match mv {
        Move::Pass => "--".to_string(),
        Move::Place(c) => {
            let col_char = (b'A' + c.col) as char;
            format!("{}{}", col_char, c.row + 1)
        }
    }
}

/// Extracts the board size from a `BO[8 ...]` value.
///
/// The expected layout is "size + whitespace + 64 board characters +
/// whitespace + side-to-move character", e.g.
/// `BO[8 ---------------------------O*------*O--------------------------- *]`.
/// Returns the default (8x8) when the format does not match.
fn parse_board_size(bo_value: &str) -> BoardSize {
    let trimmed = bo_value.trim();
    if let Some((size_str, _)) = trimmed.split_once(char::is_whitespace) {
        if let Ok(n) = size_str.parse::<u8>()
            && (4..=26).contains(&n)
        {
            return BoardSize::square(n);
        }
    }
    BoardSize::STANDARD
}

/// Computes `(winner, margin)` from the `+12` portion of `RE[+12]`. The
/// absolute value of the margin is the score differential. `?` is treated
/// as unknown.
fn parse_result(re_value: &str, total_stones_hint: Option<u32>) -> Option<GameResultRecord> {
    let trimmed = re_value.trim();
    if trimmed.is_empty() || trimmed == "?" {
        return None;
    }
    let n: i32 = trimmed.parse().ok()?;
    let total = total_stones_hint.unwrap_or(64) as i32;
    let (winner, b, w);
    if n > 0 {
        winner = Some(Color::Black);
        b = ((total + n) / 2) as u32;
        w = ((total - n) / 2) as u32;
    } else if n < 0 {
        winner = Some(Color::White);
        b = ((total + n) / 2) as u32;
        w = ((total - n) / 2) as u32;
    } else {
        winner = None;
        b = (total / 2) as u32;
        w = (total / 2) as u32;
    }
    Some(GameResultRecord {
        winner,
        score: Score { black: b, white: w },
    })
}

impl GameRecordReader for GgfReader {
    fn read_game<R: Read>(&mut self, mut source: R) -> Result<GameRecord, IoError> {
        let mut buf = String::new();
        source.read_to_string(&mut buf)?;
        let tags = parse_tags(&buf)?;

        let mut gm: Option<String> = None;
        let mut pb = "Black".to_string();
        let mut pw = "White".to_string();
        let mut re_raw: Option<String> = None;
        let mut bo_raw: Option<String> = None;
        let mut moves_raw: Vec<(Color, String)> = Vec::new();

        for tag in &tags {
            match tag.name.as_str() {
                "GM" => gm = Some(tag.value.clone()),
                "PB" => pb = tag.value.clone(),
                "PW" => pw = tag.value.clone(),
                "RE" => re_raw = Some(tag.value.clone()),
                "BO" => bo_raw = Some(tag.value.clone()),
                "B" => moves_raw.push((Color::Black, tag.value.clone())),
                "W" => moves_raw.push((Color::White, tag.value.clone())),
                _ => {} // ignore unknown tags (PC, DT, TI, ...)
            }
        }

        if let Some(gm) = &gm
            && gm.to_lowercase() != "othello"
        {
            return Err(IoError::Parse(format!("not an Othello GGF: GM={gm:?}")));
        }

        let board_size = bo_raw
            .as_deref()
            .map(parse_board_size)
            .unwrap_or(BoardSize::STANDARD);

        let total_cells = (board_size.rows as u32) * (board_size.cols as u32);

        // 着手のパース
        let now = Utc::now().with_timezone(
            &FixedOffset::east_opt(9 * 3600).unwrap_or(FixedOffset::east_opt(0).unwrap()),
        );
        let mut moves: Vec<MoveEntry> = Vec::with_capacity(moves_raw.len());
        for (n, (side, raw)) in moves_raw.into_iter().enumerate() {
            let mv = parse_ggf_move(&raw)?;
            moves.push(MoveEntry {
                n: (n + 1) as u32,
                side,
                r#move: mv,
                ts: now,
            });
        }

        let result = re_raw.and_then(|s| parse_result(&s, Some(total_cells)));

        let metadata = GameMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: now,
            ended_at: None,
            board_size,
            players: PlayerPair {
                black: PlayerInfo::just_name(pb),
                white: PlayerInfo::just_name(pw),
            },
            result,
            engine_version: format!("rs-othello-sim {}", env!("CARGO_PKG_VERSION")),
        };

        Ok(GameRecord {
            schema_version: SCHEMA_VERSION.to_string(),
            metadata,
            moves,
        })
    }
}

impl GameRecordWriter for GgfWriter {
    fn write_game<W: Write>(&mut self, mut dest: W, record: &GameRecord) -> Result<(), IoError> {
        let size = record.metadata.board_size;
        let total = (size.rows as i32) * (size.cols as i32);
        let date = record.metadata.started_at.format("%Y-%m-%d");

        // 結果の計算 ( RE タグ用)
        let re_str = match &record.metadata.result {
            None => "?".to_string(),
            Some(r) => match r.winner {
                None => "0".to_string(),
                Some(Color::Black) => format!("+{}", r.score.black as i32 - r.score.white as i32),
                Some(Color::White) => format!("{}", -(r.score.white as i32 - r.score.black as i32)),
            },
        };

        // 初期盤面 ( 標準初期配置．サイズに依らず "標準" として書き出す)
        let mut bo_chars: Vec<char> = vec!['-'; (size.rows as usize) * (size.cols as usize)];
        if size.rows >= 4 && size.cols >= 4 {
            let r_mid = (size.rows / 2) as usize;
            let c_mid = (size.cols / 2) as usize;
            // 行優先
            let idx = |r: usize, c: usize| r * (size.cols as usize) + c;
            // 標準初期配置: (mid-1, mid-1)=W, (mid-1, mid)=B, (mid, mid-1)=B, (mid, mid)=W
            bo_chars[idx(r_mid - 1, c_mid - 1)] = 'O';
            bo_chars[idx(r_mid - 1, c_mid)] = '*';
            bo_chars[idx(r_mid, c_mid - 1)] = '*';
            bo_chars[idx(r_mid, c_mid)] = 'O';
        }
        let bo_string: String = bo_chars.iter().collect();

        write!(dest, "(;GM[Othello]PC[rs-othello-sim]")?;
        write!(dest, "DT[{date}]")?;
        write!(dest, "PB[{}]", record.metadata.players.black.name)?;
        write!(dest, "PW[{}]", record.metadata.players.white.name)?;
        write!(dest, "RE[{re_str}]")?;
        write!(dest, "BO[{} {} *]", size.rows, bo_string)?;

        // sanity: total はマージン正規化で使えるが現バージョンでは未使用
        let _ = total;

        for entry in &record.moves {
            let tag = match entry.side {
                Color::Black => "B",
                Color::White => "W",
            };
            write!(dest, "{}[{}]", tag, move_to_ggf(entry.r#move))?;
        }
        write!(dest, ";)")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_move() {
        assert_eq!(parse_ggf_move("D3").unwrap(), Move::Place(Coord::new(2, 3)));
        assert_eq!(parse_ggf_move("d3").unwrap(), Move::Place(Coord::new(2, 3)));
        assert_eq!(parse_ggf_move("A1").unwrap(), Move::Place(Coord::new(0, 0)));
        assert_eq!(parse_ggf_move("H8").unwrap(), Move::Place(Coord::new(7, 7)));
    }

    #[test]
    fn parse_move_with_time() {
        assert_eq!(
            parse_ggf_move("D3//1.234").unwrap(),
            Move::Place(Coord::new(2, 3))
        );
    }

    #[test]
    fn parse_pass_variants() {
        assert_eq!(parse_ggf_move("--").unwrap(), Move::Pass);
        assert_eq!(parse_ggf_move("PA").unwrap(), Move::Pass);
        assert_eq!(parse_ggf_move("pa").unwrap(), Move::Pass);
    }

    #[test]
    fn move_to_ggf_round_trip() {
        for &(r, c) in &[(0u8, 0u8), (2, 3), (7, 7), (3, 5)] {
            let mv = Move::Place(Coord::new(r, c));
            let s = move_to_ggf(mv);
            assert_eq!(parse_ggf_move(&s).unwrap(), mv);
        }
        assert_eq!(move_to_ggf(Move::Pass), "--");
    }

    #[test]
    fn read_minimal_ggf() {
        let ggf = "(;GM[Othello]PB[Foo]PW[Bar]B[D3]W[C5];)";
        let mut reader = GgfReader::new();
        let record = reader.read_game(ggf.as_bytes()).unwrap();
        assert_eq!(record.metadata.players.black.name, "Foo");
        assert_eq!(record.metadata.players.white.name, "Bar");
        assert_eq!(record.moves.len(), 2);
        assert_eq!(record.moves[0].r#move, Move::Place(Coord::new(2, 3)));
        assert_eq!(record.moves[0].side, Color::Black);
        assert_eq!(record.moves[1].r#move, Move::Place(Coord::new(4, 2)));
        assert_eq!(record.moves[1].side, Color::White);
    }

    #[test]
    fn round_trip_moves() {
        // GGF 自身のラウンドトリップ ( 着手列のみ確認)
        use crate::record::{GameMetadata, MoveEntry, PlayerInfo, PlayerPair};
        use chrono::DateTime;
        let ts = DateTime::parse_from_rfc3339("2026-05-09T15:30:00.000+09:00").unwrap();
        let record = GameRecord::new(
            GameMetadata {
                id: "x".into(),
                started_at: ts,
                ended_at: None,
                board_size: BoardSize::STANDARD,
                players: PlayerPair {
                    black: PlayerInfo::just_name("Foo"),
                    white: PlayerInfo::just_name("Bar"),
                },
                result: None,
                engine_version: "v".into(),
            },
            vec![
                MoveEntry {
                    n: 1,
                    side: Color::Black,
                    r#move: Move::Place(Coord::new(2, 3)),
                    ts,
                },
                MoveEntry {
                    n: 2,
                    side: Color::White,
                    r#move: Move::Place(Coord::new(4, 2)),
                    ts,
                },
                MoveEntry {
                    n: 3,
                    side: Color::Black,
                    r#move: Move::Pass,
                    ts,
                },
            ],
        );
        let mut buf = Vec::new();
        GgfWriter::new().write_game(&mut buf, &record).unwrap();
        let parsed = GgfReader::new().read_game(buf.as_slice()).unwrap();
        let original_moves: Vec<_> = record.moves.iter().map(|m| (m.side, m.r#move)).collect();
        let parsed_moves: Vec<_> = parsed.moves.iter().map(|m| (m.side, m.r#move)).collect();
        assert_eq!(original_moves, parsed_moves);
    }

    #[test]
    fn pass_round_trip_via_dashes() {
        let ggf = "(;GM[Othello]PB[Foo]PW[Bar]B[--];)";
        let parsed = GgfReader::new().read_game(ggf.as_bytes()).unwrap();
        assert_eq!(parsed.moves.len(), 1);
        assert_eq!(parsed.moves[0].r#move, Move::Pass);
    }

    #[test]
    fn rejects_non_othello() {
        let ggf = "(;GM[Chess]PB[Foo]PW[Bar];)";
        let r = GgfReader::new().read_game(ggf.as_bytes());
        assert!(matches!(r, Err(IoError::Parse(_))));
    }
}
