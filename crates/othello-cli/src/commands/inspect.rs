//! `inspect` subcommand: print summary statistics for a game record.

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use othello_core::{Color, Move};
use othello_io::{GameRecord, GameRecordReader, GgfReader, JsonReader, WthorHeader, WthorReader};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// Arguments for `inspect`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Input file.
    #[arg(long)]
    pub file: PathBuf,

    /// Input format.
    #[arg(long, value_enum)]
    pub format: InspectFormat,
}

/// Input format for `inspect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectFormat {
    /// Native JSON.
    Json,
    /// GGF.
    Ggf,
    /// WTHOR binary.
    Wthor,
}

/// Entry point for `inspect`. Writes the summary to stdout.
pub fn run(args: Args) -> Result<()> {
    let mut out = std::io::stdout().lock();
    inspect_to_writer(&args, &mut out)
}

/// Core implementation with the output sink abstracted.
pub fn inspect_to_writer<W: std::io::Write>(args: &Args, out: &mut W) -> Result<()> {
    let path_display = args.file.display().to_string();
    match args.format {
        InspectFormat::Json => {
            let file = File::open(&args.file)
                .with_context(|| format!("failed to open: {path_display}"))?;
            let record = JsonReader::new()
                .read_game(BufReader::new(file))
                .with_context(|| "failed to parse JSON")?;
            write_single_summary(out, &path_display, "json", &record)?;
        }
        InspectFormat::Ggf => {
            let file = File::open(&args.file)
                .with_context(|| format!("failed to open: {path_display}"))?;
            let record = GgfReader::new()
                .read_game(BufReader::new(file))
                .with_context(|| "failed to parse GGF")?;
            write_single_summary(out, &path_display, "ggf", &record)?;
        }
        InspectFormat::Wthor => {
            let file = File::open(&args.file)
                .with_context(|| format!("failed to open: {path_display}"))?;
            let mut wthor = WthorReader::new(BufReader::new(file))
                .with_context(|| "failed to read WTHOR header")?;
            let header = *wthor.header();
            let records = wthor.read_all().with_context(|| "failed to read WTHOR")?;
            write_wthor_summary(out, &path_display, &header, &records)?;
        }
    }
    Ok(())
}

fn write_single_summary<W: std::io::Write>(
    out: &mut W,
    path: &str,
    format: &str,
    record: &GameRecord,
) -> Result<()> {
    let pass_count = record
        .moves
        .iter()
        .filter(|m| matches!(m.r#move, Move::Pass))
        .count();
    writeln!(out, "File:        {path}")?;
    writeln!(out, "Format:      {format}")?;
    writeln!(out, "Game ID:     {}", record.metadata.id)?;
    writeln!(
        out,
        "Board:       {}x{}",
        record.metadata.board_size.rows, record.metadata.board_size.cols
    )?;
    writeln!(
        out,
        "Started:     {}",
        record.metadata.started_at.to_rfc3339()
    )?;
    writeln!(
        out,
        "Ended:       {}",
        record
            .metadata
            .ended_at
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "-".into())
    )?;
    writeln!(
        out,
        "Players:     Black={} ({})  White={} ({})",
        record.metadata.players.black.name,
        compact_params(&record.metadata.players.black.params),
        record.metadata.players.white.name,
        compact_params(&record.metadata.players.white.params),
    )?;
    writeln!(
        out,
        "Moves:       {} (Pass count: {pass_count})",
        record.moves.len()
    )?;
    if let Some(r) = record.metadata.result.as_ref() {
        let winner = match r.winner {
            None => "Draw".to_string(),
            Some(Color::Black) => "Black".to_string(),
            Some(Color::White) => "White".to_string(),
        };
        writeln!(
            out,
            "Result:      Winner={winner}  Score: Black={} White={}",
            r.score.black, r.score.white
        )?;
    } else {
        writeln!(out, "Result:      (in progress)")?;
    }
    Ok(())
}

fn write_wthor_summary<W: std::io::Write>(
    out: &mut W,
    path: &str,
    header: &WthorHeader,
    records: &[GameRecord],
) -> Result<()> {
    let total = records.len();
    writeln!(out, "File:        {path}")?;
    writeln!(out, "Format:      wthor")?;
    writeln!(out, "Games:       {total}")?;
    writeln!(out, "Year:        {}", header.year)?;
    writeln!(out, "Board:       {0}x{0}", header.board_size)?;

    if total == 0 {
        return Ok(());
    }

    let avg_moves = records.iter().map(|r| r.moves.len()).sum::<usize>() as f64 / total as f64;
    let mut bw = 0usize;
    let mut ww = 0usize;
    let mut dr = 0usize;
    for rec in records {
        if let Some(r) = rec.metadata.result.as_ref() {
            match r.winner {
                Some(Color::Black) => bw += 1,
                Some(Color::White) => ww += 1,
                None => dr += 1,
            }
        }
    }
    let pct = |n: usize| -> f64 { (n as f64) * 100.0 / (total as f64) };
    writeln!(out, "Avg moves:   {avg_moves:.1}")?;
    writeln!(
        out,
        "Black wins:  {:.1}%  White wins: {:.1}%  Draws: {:.1}%",
        pct(bw),
        pct(ww),
        pct(dr)
    )?;
    Ok(())
}

fn compact_params(v: &serde_json::Value) -> String {
    if v.is_null() || (v.is_object() && v.as_object().is_some_and(|m| m.is_empty())) {
        return String::new();
    }
    serde_json::to_string(v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_json_writes_summary() {
        let dir = std::env::temp_dir().join(format!(
            "rs-othello-sim-inspect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("g.json");
        let json = r#"{
            "schema_version": "1.0",
            "metadata": {
                "id": "id-1",
                "started_at": "2026-05-09T15:30:00.000+09:00",
                "ended_at": "2026-05-09T15:30:42.000+09:00",
                "board_size": {"rows": 8, "cols": 8},
                "players": {
                    "black": {"name": "Mcts", "params": {"sim": 1000}},
                    "white": {"name": "Random", "params": {"seed": 42}}
                },
                "result": {"winner": "Black", "score": {"black": 38, "white": 26}},
                "engine_version": "v"
            },
            "moves": [
                {"n": 1, "side": "Black", "move": {"Place": {"row": 2, "col": 3}}, "ts": "2026-05-09T15:30:00.000+09:00"}
            ]
        }"#;
        std::fs::write(&path, json).unwrap();
        let mut buf = Vec::new();
        inspect_to_writer(
            &Args {
                file: path,
                format: InspectFormat::Json,
            },
            &mut buf,
        )
        .unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Format:      json"));
        assert!(s.contains("Game ID:     id-1"));
        assert!(s.contains("Board:       8x8"));
        assert!(s.contains("Black=Mcts"));
        assert!(s.contains("White=Random"));
        assert!(s.contains("Result:      Winner=Black"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
