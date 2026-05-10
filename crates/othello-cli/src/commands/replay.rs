//! `replay` subcommand: load a game record and replay it in the TUI.

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use othello_io::{GameRecordReader, GgfReader, JsonReader};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// Arguments for `replay`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Game-record file.
    #[arg(long)]
    pub file: PathBuf,

    /// Game-record format.
    #[arg(long, value_enum, default_value_t = ReplayFormat::Json)]
    pub format: ReplayFormat,

    /// Start auto-play immediately on launch.
    #[arg(long, default_value_t = false)]
    pub auto: bool,

    /// Auto-play step interval in milliseconds. Adjustable from Replay mode with `+` / `-`.
    #[arg(long, default_value_t = 500)]
    pub auto_delay: u64,
}

/// Input format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReplayFormat {
    /// Native JSON.
    Json,
    /// GGF.
    Ggf,
}

/// Entry point for `replay`.
pub fn run(args: Args) -> Result<()> {
    let file = File::open(&args.file)
        .with_context(|| format!("failed to open: {}", args.file.display()))?;
    let mut reader = BufReader::new(file);
    let record = match args.format {
        ReplayFormat::Json => JsonReader::new()
            .read_game(&mut reader)
            .with_context(|| "failed to read JSON record")?,
        ReplayFormat::Ggf => GgfReader::new()
            .read_game(&mut reader)
            .with_context(|| "failed to read GGF record")?,
    };
    let history = othello_tui::record_to_history(&record)?;
    let options = othello_tui::ReplayOptions {
        auto_play: args.auto,
        auto_delay_ms: args.auto_delay,
    };
    othello_tui::run_replay_with_options(history, options)?;
    Ok(())
}
