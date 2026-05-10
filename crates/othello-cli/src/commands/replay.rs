//! `replay` サブコマンド: 棋譜を読み込んで TUI で再生する．

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use othello_io::{GameRecordReader, GgfReader, JsonReader};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// `replay` の引数．
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// 棋譜ファイル．
    #[arg(long)]
    pub file: PathBuf,

    /// 棋譜フォーマット．
    #[arg(long, value_enum, default_value_t = ReplayFormat::Json)]
    pub format: ReplayFormat,

    /// 起動直後から自動再生を開始する．
    #[arg(long, default_value_t = false)]
    pub auto: bool,

    /// 自動再生の手間隔 ( ミリ秒)．Replay モード内で `+` / `-` で調整可能．
    #[arg(long, default_value_t = 500)]
    pub auto_delay: u64,
}

/// 入力フォーマット．
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReplayFormat {
    /// 自前 JSON．
    Json,
    /// GGF．
    Ggf,
}

/// `replay` 実行関数．
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
