//! `convert` subcommand: convert game records (WTHOR/JSON/GGF -> JSON/GGF).

use anyhow::{Context, Result, anyhow};
use clap::{Args as ClapArgs, ValueEnum};
use othello_io::{
    GameRecord, GameRecordReader, GameRecordWriter, GgfReader, GgfWriter, JsonReader, JsonWriter,
    WthorReader,
};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Input format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InputFormat {
    /// Native JSON.
    Json,
    /// GGF.
    Ggf,
    /// WTHOR binary.
    Wthor,
}

/// Output format (WTHOR cannot be written).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Native JSON.
    Json,
    /// GGF.
    Ggf,
}

/// Arguments for `convert`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Input file.
    #[arg(long)]
    pub input: PathBuf,

    /// Input format.
    #[arg(long, value_enum)]
    pub input_format: InputFormat,

    /// Output format.
    #[arg(long, value_enum)]
    pub output_format: OutputFormat,

    /// Output file for a single record (mutually exclusive with --output-dir).
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Output directory for multiple records (mutually exclusive with --output).
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
}

/// Entry point for `convert`.
pub fn run(args: Args) -> Result<()> {
    if args.output.is_some() && args.output_dir.is_some() {
        return Err(anyhow!("--output and --output-dir are mutually exclusive"));
    }

    let records = read_records(&args.input, args.input_format)?;

    if records.is_empty() {
        println!("No records found in input.");
        return Ok(());
    }

    if let Some(path) = args.output.as_ref() {
        if records.len() == 1 {
            write_record(path, args.output_format, &records[0])?;
            println!("Wrote 1 record to: {}", path.display());
            return Ok(());
        }
    }

    if let Some(dir) = args.output_dir.as_ref() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create output directory: {}", dir.display()))?;
        let ext = match args.output_format {
            OutputFormat::Json => "json",
            OutputFormat::Ggf => "ggf",
        };
        for (i, record) in records.iter().enumerate() {
            let name = format!("game_{:04}.{ext}", i + 1);
            let path = dir.join(name);
            write_record(&path, args.output_format, record)?;
        }
        println!("Wrote {} records to: {}", records.len(), dir.display());
        return Ok(());
    }

    if records.len() == 1 {
        return Err(anyhow!(
            "single-record input requires --output FILE (got none)"
        ));
    }

    Err(anyhow!(
        "multi-record input requires --output-dir DIR (got {} records)",
        records.len()
    ))
}

fn read_records(path: &Path, format: InputFormat) -> Result<Vec<GameRecord>> {
    let file = File::open(path).with_context(|| format!("failed to open: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    match format {
        InputFormat::Json => {
            let r = JsonReader::new()
                .read_game(&mut reader)
                .with_context(|| "failed to read JSON")?;
            Ok(vec![r])
        }
        InputFormat::Ggf => {
            let r = GgfReader::new()
                .read_game(&mut reader)
                .with_context(|| "failed to read GGF")?;
            Ok(vec![r])
        }
        InputFormat::Wthor => {
            let mut wthor =
                WthorReader::new(&mut reader).with_context(|| "failed to read WTHOR header")?;
            wthor
                .read_all()
                .with_context(|| "failed to read WTHOR records")
        }
    }
}

fn write_record(path: &Path, format: OutputFormat, record: &GameRecord) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create: {}", path.display()))?;
    let mut buf = BufWriter::new(file);
    match format {
        OutputFormat::Json => JsonWriter::new()
            .write_game(&mut buf, record)
            .with_context(|| "JSON write failed")?,
        OutputFormat::Ggf => GgfWriter::new()
            .write_game(&mut buf, record)
            .with_context(|| "GGF write failed")?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::{Color, Coord, Move};

    /// Builds a minimal WTHOR byte sequence for the convert tests.
    fn make_wthor_bytes() -> Vec<u8> {
        let mut header = [0u8; 16];
        header[4..8].copy_from_slice(&1u32.to_le_bytes()); // n_games
        header[8..10].copy_from_slice(&2023u16.to_le_bytes()); // year
        header[10] = 8;
        header[11] = 0;

        let mut block = [0u8; 68];
        block[0..2].copy_from_slice(&0u16.to_le_bytes());
        block[2..4].copy_from_slice(&100u16.to_le_bytes()); // black_id
        block[4..6].copy_from_slice(&200u16.to_le_bytes()); // white_id
        block[6] = 32; // real_score
        block[7] = 32;
        // moves: D3 (2,3) → byte = 3*10 + 4 = 34
        block[8] = 34;

        let mut data = Vec::new();
        data.extend_from_slice(&header);
        data.extend_from_slice(&block);
        data
    }

    #[test]
    fn convert_wthor_to_json_roundtrip() {
        // 一時ディレクトリを使って WTHOR → JSON 変換を確認．
        let dir = std::env::temp_dir().join(format!(
            "rs-othello-sim-convert-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let input_path = dir.join("in.wtb");
        std::fs::write(&input_path, make_wthor_bytes()).unwrap();

        let out_dir = dir.join("out");
        let args = Args {
            input: input_path,
            input_format: InputFormat::Wthor,
            output_format: OutputFormat::Json,
            output: None,
            output_dir: Some(out_dir.clone()),
        };
        run(args).unwrap();

        let entries: Vec<_> = std::fs::read_dir(&out_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let json_path = out_dir.join("game_0001.json");
        assert!(json_path.exists());

        // 読み戻して move が一致
        let file = std::fs::File::open(&json_path).unwrap();
        let parsed = JsonReader::new().read_game(file).unwrap();
        assert_eq!(parsed.moves.len(), 1);
        assert_eq!(parsed.moves[0].r#move, Move::Place(Coord::new(2, 3)));
        assert_eq!(parsed.moves[0].side, Color::Black);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn convert_single_json_to_ggf() {
        let dir = std::env::temp_dir().join(format!(
            "rs-othello-sim-convert2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        // 最小限の JSON 棋譜を書く
        let input_path = dir.join("in.json");
        let json = r#"{
            "schema_version": "1.0",
            "metadata": {
                "id": "test",
                "started_at": "2026-05-09T15:30:00.000+09:00",
                "ended_at": null,
                "board_size": {"rows": 8, "cols": 8},
                "players": {
                    "black": {"name": "A", "params": {}},
                    "white": {"name": "B", "params": {}}
                },
                "result": null,
                "engine_version": "v"
            },
            "moves": [
                {"n": 1, "side": "Black", "move": {"Place": {"row": 2, "col": 3}}, "ts": "2026-05-09T15:30:00.123+09:00"}
            ]
        }"#;
        std::fs::write(&input_path, json).unwrap();
        let out_path = dir.join("out.ggf");
        let args = Args {
            input: input_path,
            input_format: InputFormat::Json,
            output_format: OutputFormat::Ggf,
            output: Some(out_path.clone()),
            output_dir: None,
        };
        run(args).unwrap();
        let s = std::fs::read_to_string(&out_path).unwrap();
        assert!(s.starts_with("(;GM[Othello]"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_both_output_options() {
        let args = Args {
            input: PathBuf::from("dummy"),
            input_format: InputFormat::Json,
            output_format: OutputFormat::Json,
            output: Some(PathBuf::from("a.json")),
            output_dir: Some(PathBuf::from("d")),
        };
        let r = run(args);
        assert!(r.is_err());
    }
}
