//! `simulate` subcommand: simulate a single game (random/greedy/etc.).

use crate::player_spec_with_nn::build_player;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use clap::ValueEnum;
use othello_core::{BoardSize, Color};
use othello_engine::{EngineConfig, GameEngine};
use othello_io::{GameRecordWriter, GgfWriter, JsonWriter, JsonlLogger, PlayerInfo, PlayerPair};
use othello_player::Player;
use othello_player::player_spec::{parse_player_spec, spec_name, spec_params};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

/// Game-record output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RecordFormat {
    /// Native JSON format.
    Json,
    /// GGF format.
    Ggf,
}

/// Arguments for `othello-cli simulate`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Board size (4..=26).
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// Black player SPEC (e.g. `random:seed=42`, `greedy`).
    #[arg(long, default_value = "random")]
    pub black: String,

    /// White player SPEC.
    #[arg(long, default_value = "random")]
    pub white: String,

    /// Output file for the saved game record (omit to skip saving).
    #[arg(long)]
    pub save_record: Option<PathBuf>,

    /// Game-record output format.
    #[arg(long, value_enum, default_value_t = RecordFormat::Json)]
    pub record_format: RecordFormat,

    /// Output path for the JSONL move-by-move log (omit to skip).
    #[arg(long)]
    pub jsonl_log: Option<PathBuf>,
}

/// Entry point for `simulate`.
pub fn run(args: Args) -> Result<()> {
    let size = BoardSize::square(args.board_size);

    let black_spec = parse_player_spec(&args.black)
        .with_context(|| format!("invalid --black: {:?}", args.black))?;
    let white_spec = parse_player_spec(&args.white)
        .with_context(|| format!("invalid --white: {:?}", args.white))?;

    let mut black = build_player(&black_spec, Color::Black)?;
    let mut white = build_player(&white_spec, Color::White)?;

    let players = PlayerPair {
        black: PlayerInfo {
            name: spec_name(&black_spec).to_string(),
            params: spec_params(&black_spec),
        },
        white: PlayerInfo {
            name: spec_name(&white_spec).to_string(),
            params: spec_params(&white_spec),
        },
    };

    let mut cfg = EngineConfig::with_size(size);
    if let Some(path) = &args.jsonl_log {
        let logger = JsonlLogger::to_path(path)
            .with_context(|| format!("failed to open jsonl log: {}", path.display()))?;
        cfg.jsonl_logger = Some(logger);
    }
    let mut engine =
        GameEngine::new(cfg).with_context(|| format!("failed to construct engine for {size:?}"))?;

    let result = run_engine(&mut engine, black.as_mut(), white.as_mut(), players.clone())?;

    println!(
        "Game over: black={} white={} winner={:?} total_moves={}",
        result.black, result.white, result.winner, result.total_moves
    );

    if let Some(path) = args.save_record {
        let record = engine.into_record(players);
        let file = File::create(&path)
            .with_context(|| format!("failed to create record file: {}", path.display()))?;
        let mut buf = BufWriter::new(file);
        match args.record_format {
            RecordFormat::Json => JsonWriter::new()
                .write_game(&mut buf, &record)
                .context("JSON write failed")?,
            RecordFormat::Ggf => GgfWriter::new()
                .write_game(&mut buf, &record)
                .context("GGF write failed")?,
        }
        println!("Record saved to: {}", path.display());
    }

    if let Some(path) = &args.jsonl_log {
        println!("JSONL log saved to: {}", path.display());
    }

    Ok(())
}

/// Thin wrapper that takes two `Box<dyn Player>` and calls `GameEngine::run_with_meta`.
fn run_engine(
    engine: &mut GameEngine,
    black: &mut dyn Player,
    white: &mut dyn Player,
    players: PlayerPair,
) -> Result<othello_core::GameResult> {
    struct Adapter<'a>(&'a mut dyn Player);
    impl Player for Adapter<'_> {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn color(&self) -> Color {
            self.0.color()
        }
        fn select_move(
            &mut self,
            state: &othello_core::GameState,
        ) -> std::result::Result<othello_core::Move, othello_player::PlayerError> {
            self.0.select_move(state)
        }
        fn on_game_end(
            &mut self,
            final_state: &othello_core::GameState,
            result: othello_core::GameResult,
        ) {
            self.0.on_game_end(final_state, result)
        }
        fn reset(&mut self) {
            self.0.reset()
        }
    }
    let mut b = Adapter(black);
    let mut w = Adapter(white);
    engine
        .run_with_meta(&mut b, &mut w, players)
        .map_err(|e| anyhow::anyhow!(e))
}
