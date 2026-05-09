//! `observe` サブコマンド: AI 同士の対戦を TUI で観戦する．

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use othello_core::BoardSize;
use othello_player::player_spec::parse_player_spec;

/// `othello-cli observe` の引数．
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// 盤面サイズ．
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// 黒プレイヤー SPEC．
    #[arg(long, default_value = "mcts:200")]
    pub black: String,

    /// 白プレイヤー SPEC．
    #[arg(long, default_value = "greedy")]
    pub white: String,

    /// 乱数 seed．
    #[arg(long)]
    pub seed: Option<u64>,

    /// 自動再生間隔 ( ms)．0 だと手動 ( Space で 1 手進める)．
    #[arg(long, default_value_t = 0u64)]
    pub auto_delay: u64,
}

/// `observe` 実行関数．
pub fn run(args: Args) -> Result<()> {
    let size = BoardSize::square(args.board_size);
    let black_spec = parse_player_spec(&args.black)
        .with_context(|| format!("invalid --black: {:?}", args.black))?;
    let white_spec = parse_player_spec(&args.white)
        .with_context(|| format!("invalid --white: {:?}", args.white))?;
    let seed = args.seed.unwrap_or(0);
    let cfg = othello_tui::ObserveConfig {
        board_size: size,
        black_spec,
        white_spec,
        seed,
        auto_delay_ms: args.auto_delay,
    };
    othello_tui::run_observe(cfg)?;
    Ok(())
}
