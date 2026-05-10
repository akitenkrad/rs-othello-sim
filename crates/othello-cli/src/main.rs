//! `othello-cli` バイナリエントリポイント．

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

/// rs-othello-sim CLI フロントエンド．
#[derive(Debug, Parser)]
#[command(name = "othello-cli", version, about = "Othello simulator CLI")]
struct Cli {
    /// tracing ログレベル ( trace/debug/info/warn/error)．stderr に適用される．
    #[arg(long, global = true, default_value = "info")]
    log_level: String,

    /// tracing 出力フォーマット ( text / json)．stderr 出力のみに適用．
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,

    /// 追加で JSON 形式のログをファイルに出力する ( stderr 出力と並行)．
    #[arg(long, global = true)]
    log_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

/// tracing 出力フォーマット．
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LogFormat {
    /// 人間可読 ( デフォルト)．
    Text,
    /// JSON 構造化．
    Json,
}

/// サブコマンド一覧．
#[derive(Debug, Subcommand)]
enum Command {
    /// 2 人対戦 ( 標準入出力)．
    Play(commands::play::Args),
    /// 1 局のシミュレーション ( random/greedy 等)．
    Simulate(commands::simulate::Args),
    /// 棋譜の TUI 再生．
    Replay(commands::replay::Args),
    /// 棋譜形式変換 ( WTHOR/JSON/GGF → JSON/GGF)．
    Convert(commands::convert::Args),
    /// 棋譜の統計情報を表示．
    Inspect(commands::inspect::Args),
    /// バッチ self-play ( BatchRunner)．
    Selfplay(commands::selfplay::Args),
    /// 簡易ベンチマーク ( legal-moves / self-play)．
    Benchmark(commands::benchmark::Args),
    /// AI 対戦の TUI 観戦．
    Observe(commands::observe::Args),
    /// 公開データセット ( WTHOR 等) のダウンロード．
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
