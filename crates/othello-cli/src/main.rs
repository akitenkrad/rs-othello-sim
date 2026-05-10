//! `othello-cli` binary entry point.

mod commands;
pub mod player_spec_with_nn;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// rs-othello-sim CLI frontend.
#[derive(Debug, Parser)]
#[command(name = "othello-cli", version, about = "Othello simulator CLI")]
struct Cli {
    /// tracing log level (trace/debug/info/warn/error). Applied to stderr.
    #[arg(long, global = true, default_value = "info")]
    log_level: String,

    /// tracing output format (text/json). Applied to stderr only.
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,

    /// Additionally write JSON-formatted logs to a file (alongside stderr).
    #[arg(long, global = true)]
    log_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

/// tracing output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LogFormat {
    /// Human-readable (default).
    Text,
    /// Structured JSON.
    Json,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Two-player match (stdin/stdout).
    Play(commands::play::Args),
    /// Simulate one game (random/greedy/etc.).
    Simulate(commands::simulate::Args),
    /// Replay a game record in the TUI.
    Replay(commands::replay::Args),
    /// Convert game records (WTHOR/JSON/GGF -> JSON/GGF).
    Convert(commands::convert::Args),
    /// Print summary statistics for a game record.
    Inspect(commands::inspect::Args),
    /// Batch self-play (BatchRunner).
    Selfplay(commands::selfplay::Args),
    /// Simple benchmarks (legal-moves / self-play).
    Benchmark(commands::benchmark::Args),
    /// Watch an AI vs AI match in the TUI.
    Observe(commands::observe::Args),
    /// Download public datasets (e.g. WTHOR).
    Fetch(commands::fetch::Args),
}

fn init_tracing(level: &str, format: LogFormat, log_file: Option<&PathBuf>) -> Result<()> {
    let filter_for_stderr =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let filter_for_file = EnvFilter::new(level);

    // stderr layer
    let stderr_layer: Box<dyn tracing_subscriber::Layer<_> + Send + Sync> = match format {
        LogFormat::Text => Box::new(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(filter_for_stderr),
        ),
        LogFormat::Json => Box::new(
            fmt::layer()
                .json()
                .with_writer(std::io::stderr)
                .with_filter(filter_for_stderr),
        ),
    };

    let registry = tracing_subscriber::registry().with(stderr_layer);

    if let Some(path) = log_file {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).ok();
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let file_layer = fmt::layer()
            .json()
            .with_writer(file)
            .with_filter(filter_for_file);
        let _ = registry.with(file_layer).try_init();
    } else {
        let _ = registry.try_init();
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level, cli.log_format, cli.log_file.as_ref())?;
    match cli.command {
        Command::Play(args) => commands::play::run(args),
        Command::Simulate(args) => commands::simulate::run(args),
        Command::Replay(args) => commands::replay::run(args),
        Command::Convert(args) => commands::convert::run(args),
        Command::Inspect(args) => commands::inspect::run(args),
        Command::Selfplay(args) => commands::selfplay::run(args),
        Command::Benchmark(args) => commands::benchmark::run(args),
        Command::Observe(args) => commands::observe::run(args),
        Command::Fetch(args) => commands::fetch::run(args),
    }
}
