//! `othello-cli` バイナリエントリポイント．

mod commands;
mod player_spec;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// rs-othello-sim CLI フロントエンド．
#[derive(Debug, Parser)]
#[command(name = "othello-cli", version, about = "Othello simulator CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// サブコマンド一覧．Phase 2 では `play` / `simulate` のみ実装．
#[derive(Debug, Subcommand)]
enum Command {
    /// 2 人対戦 ( 標準入出力)．
    Play(commands::play::Args),
    /// 1 局のシミュレーション ( random/greedy 等)．
    Simulate(commands::simulate::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Play(args) => commands::play::run(args),
        Command::Simulate(args) => commands::simulate::run(args),
    }
}
