//! `observe` subcommand: watch an AI vs AI match in the TUI.

use crate::player_spec_with_nn::build_with_seed_override;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use othello_core::{BoardSize, Color};
use othello_player::player_spec::{parse_player_spec, spec_name};

/// Arguments for `othello-cli observe`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Board size.
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// Black player SPEC.
    #[arg(long, default_value = "mcts:200")]
    pub black: String,

    /// White player SPEC.
    #[arg(long, default_value = "greedy")]
    pub white: String,

    /// Random seed.
    #[arg(long)]
    pub seed: Option<u64>,

    /// Auto-play interval in milliseconds. `0` disables auto-play (advance
    /// one move at a time with Space).
    #[arg(long, default_value_t = 0u64)]
    pub auto_delay: u64,
}

/// Entry point for `observe`.
pub fn run(args: Args) -> Result<()> {
    let size = BoardSize::square(args.board_size);
    let black_spec = parse_player_spec(&args.black)
        .with_context(|| format!("invalid --black: {:?}", args.black))?;
    let white_spec = parse_player_spec(&args.white)
        .with_context(|| format!("invalid --white: {:?}", args.white))?;
    let seed = args.seed.unwrap_or(0);

    // CLI 側で Player を構築 ( Nn を含む SPEC でも対応可能)．
    let black = build_with_seed_override(&black_spec, Color::Black, seed)
        .with_context(|| "failed to build black player")?;
    let white = build_with_seed_override(&white_spec, Color::White, seed.wrapping_add(0x9E37_79B9))
        .with_context(|| "failed to build white player")?;

    let black_name = spec_name(&black_spec).to_string();
    let white_name = spec_name(&white_spec).to_string();

    othello_tui::run_observe_with_players(
        size,
        black,
        white,
        black_name,
        white_name,
        args.auto_delay,
    )?;
    Ok(())
}
