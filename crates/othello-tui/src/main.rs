//! `othello-tui` binary entry point.
//!
//! ```bash
//! othello-tui play --board-size 8
//! othello-tui replay --file game.json --format json
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use othello_core::BoardSize;
use othello_io::{GameRecordReader, GgfReader, JsonReader};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "othello-tui",
    version,
    about = "Othello TUI frontend (play/replay)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Two-player match (Human vs Human).
    Play(PlayArgs),
    /// Replay a game record.
    Replay(ReplayArgs),
}

#[derive(Debug, clap::Args)]
struct PlayArgs {
    /// Board size (4..=26).
    #[arg(long, default_value_t = 8)]
    board_size: u8,
}

#[derive(Debug, clap::Args)]
struct ReplayArgs {
    /// Game-record file.
    #[arg(long)]
    file: PathBuf,

    /// Game-record format.
    #[arg(long, value_enum, default_value_t = ReplayFormat::Json)]
    format: ReplayFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ReplayFormat {
    /// Native JSON.
    Json,
    /// GGF.
    Ggf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Play(args) => {
            let size = BoardSize::square(args.board_size);
            othello_tui::run_play(size)?;
        }
        Command::Replay(args) => {
            let file = File::open(&args.file)
                .with_context(|| format!("failed to open {}", args.file.display()))?;
            let mut reader = BufReader::new(file);
            let record = match args.format {
                ReplayFormat::Json => JsonReader::new()
                    .read_game(&mut reader)
                    .with_context(|| "failed to parse JSON record")?,
                ReplayFormat::Ggf => GgfReader::new()
                    .read_game(&mut reader)
                    .with_context(|| "failed to parse GGF record")?,
            };
            let history = othello_tui::record_to_history(&record)?;
            othello_tui::run_replay(history)?;
        }
    }
    Ok(())
}
