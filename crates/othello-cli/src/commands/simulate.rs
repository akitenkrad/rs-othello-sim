//! `simulate` サブコマンド: 1 局のシミュレーション ( random / greedy)．

use crate::player_spec::{build_player, parse_player_spec, spec_name, spec_params};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use clap::ValueEnum;
use othello_core::{BoardSize, Color};
use othello_engine::{EngineConfig, GameEngine};
use othello_io::{GameRecordWriter, GgfWriter, JsonWriter, JsonlLogger, PlayerInfo, PlayerPair};
use othello_player::Player;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

/// 棋譜出力フォーマット．
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RecordFormat {
    /// 自前 JSON 形式．
    Json,
    /// GGF 形式．
    Ggf,
}

/// `othello-cli simulate` の引数．
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// 盤面サイズ ( 4..=26)．
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// 黒プレイヤー指定 SPEC ( 例 `random:seed=42`，`greedy`)．
    #[arg(long, default_value = "random")]
    pub black: String,

    /// 白プレイヤー指定 SPEC．
    #[arg(long, default_value = "random")]
    pub white: String,

    /// 棋譜の保存先ファイル ( 未指定なら保存しない)．
    #[arg(long)]
    pub save_record: Option<PathBuf>,

    /// 棋譜のフォーマット．
    #[arg(long, value_enum, default_value_t = RecordFormat::Json)]
    pub record_format: RecordFormat,

    /// JSONL 進行ログの出力先 ( 未指定なら書き出さない)．
    #[arg(long)]
    pub jsonl_log: Option<PathBuf>,
}

/// `simulate` 実行関数．
pub fn run(args: Args) -> Result<()> {
    let size = BoardSize::square(args.board_size);

    let black_spec = parse_player_spec(&args.black)
        .with_context(|| format!("invalid --black: {:?}", args.black))?;
    let white_spec = parse_player_spec(&args.white)
        .with_context(|| format!("invalid --white: {:?}", args.white))?;

    let mut black = build_player(&black_spec, Color::Black);
    let mut white = build_player(&white_spec, Color::White);

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

/// `Box<dyn Player>` を 2 つ受け取って `GameEngine::run_with_meta` を呼ぶ薄いラッパ．
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
