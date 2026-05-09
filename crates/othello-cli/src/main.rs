//! `othello-cli` バイナリエントリポイント．

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

/// rs-othello-sim CLI フロントエンド．
#[derive(Debug, Parser)]
#[command(name = "othello-cli", version, about = "Othello simulator CLI")]
struct Cli {
    /// tracing ログレベル ( trace/debug/info/warn/error)．
    #[arg(long, global = true, default_value = "info")]
    log_level: String,

    /// tracing 出力フォーマット ( text / json)．
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Text)]
    log_format: LogFormat,

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
}

fn init_tracing(level: &str, format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);
    match format {
        LogFormat::Text => {
            let _ = builder.try_init();
        }
        LogFormat::Json => {
            let _ = builder.json().try_init();
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level, cli.log_format);
    match cli.command {
        Command::Play(args) => commands::play::run(args),
        Command::Simulate(args) => commands::simulate::run(args),
        Command::Replay(args) => commands::replay::run(args),
        Command::Convert(args) => commands::convert::run(args),
        Command::Inspect(args) => commands::inspect::run(args),
        Command::Selfplay(args) => commands::selfplay::run(args),
        Command::Benchmark(args) => commands::benchmark::run(args),
        Command::Observe(args) => commands::observe::run(args),
    }
}
