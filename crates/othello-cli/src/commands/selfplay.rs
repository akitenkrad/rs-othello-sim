//! `selfplay` subcommand: run batch self-play via `BatchRunner`.

use anyhow::{Context, Result};
use chrono::Local;
use clap::{Args as ClapArgs, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use othello_core::{BoardSize, Color};
use othello_engine::{BatchConfig, BatchRunner, GameSummary, ProgressCallback};
use othello_player::Player;
use othello_player::player_spec::{PlayerSpec, parse_player_spec};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;

/// Game-record output format for selfplay (currently JSON only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SaveRecordsFormat {
    /// Native JSON format (one file per game).
    Json,
}

/// Arguments for `othello-cli selfplay`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Board size.
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// Number of games to play.
    #[arg(long, default_value_t = 100)]
    pub num_games: usize,

    /// Number of parallel threads (0 = use all logical cores).
    #[arg(long, default_value_t = 0)]
    pub threads: usize,

    /// Black player SPEC (e.g. `random:seed=1`, `greedy`, `mcts:200`).
    #[arg(long, default_value = "random")]
    pub black: String,

    /// White player SPEC.
    #[arg(long, default_value = "random")]
    pub white: String,

    /// Random seed (each game receives `seed + game_index`).
    #[arg(long)]
    pub seed: Option<u64>,

    /// Directory in which to save the per-game JSON records.
    /// Pass `auto` to auto-generate `runs/selfplay_YYYYMMDD_HHMMSS/`.
    #[arg(long)]
    pub log_dir: Option<String>,

    /// Path of the aggregated JSONL log spanning all games.
    #[arg(long)]
    pub jsonl_log: Option<PathBuf>,

    /// Swap black and white on alternate game indices.
    #[arg(long, default_value_t = false)]
    pub swap_colors: bool,

    /// Game-record format used when `--log-dir` is set.
    #[arg(long, value_enum, default_value_t = SaveRecordsFormat::Json)]
    pub save_records: SaveRecordsFormat,

    /// Safety cap: abort with an error if a game exceeds this many moves.
    #[arg(long)]
    pub max_moves: Option<u32>,

    /// Suppress the progress bar (otherwise shown when stderr is a TTY).
    #[arg(long, default_value_t = false)]
    pub no_progress: bool,
}

/// [`ProgressCallback`] implementation backed by `indicatif::ProgressBar`.
struct IndicatifProgress {
    bar: ProgressBar,
}

impl ProgressCallback for IndicatifProgress {
    fn on_game_complete(&self, _game_index: usize, summary: &GameSummary) {
        let winner = match summary.winner {
            Some(Color::Black) => "B",
            Some(Color::White) => "W",
            None => "D",
        };
        self.bar.inc(1);
        self.bar.set_message(format!(
            "last={winner} score={}-{}",
            summary.black_score, summary.white_score
        ));
    }
}

/// Entry point for `selfplay`.
pub fn run(args: Args) -> Result<()> {
    let board_size = BoardSize::square(args.board_size);

    let black_spec = parse_player_spec(&args.black)
        .with_context(|| format!("invalid --black: {:?}", args.black))?;
    let white_spec = parse_player_spec(&args.white)
        .with_context(|| format!("invalid --white: {:?}", args.white))?;

    // log_dir 解決．`auto` なら runs/selfplay_TS/．
    let log_dir: Option<PathBuf> = match args.log_dir.as_deref() {
        Some("auto") => Some(PathBuf::from(format!(
            "runs/selfplay_{}",
            Local::now().format("%Y%m%d_%H%M%S")
        ))),
        Some(path) => Some(PathBuf::from(path)),
        None => None,
    };

    // 進捗バー: stderr が tty かつ --no-progress なしの場合のみ有効
    let show_progress = !args.no_progress && std::io::stderr().is_terminal();
    let (progress_cb, progress_bar): (Option<Arc<dyn ProgressCallback>>, Option<ProgressBar>) =
        if show_progress && args.num_games > 0 {
            let bar = ProgressBar::new(args.num_games as u64);
            bar.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("##-"),
            );
            let cb: Arc<dyn ProgressCallback> = Arc::new(IndicatifProgress { bar: bar.clone() });
            (Some(cb), Some(bar))
        } else {
            (None, None)
        };

    let cfg = BatchConfig {
        num_games: args.num_games,
        num_threads: args.threads,
        seed: args.seed,
        log_dir: log_dir.clone(),
        swap_colors: args.swap_colors,
        board_size,
        max_moves: args.max_moves,
        jsonl_log_path: args.jsonl_log.clone(),
        progress: progress_cb,
    };

    eprintln!(
        "Self-play: board={}x{} games={} threads={} swap_colors={}",
        args.board_size, args.board_size, args.num_games, args.threads, args.swap_colors
    );

    let runner = BatchRunner::new(cfg);
    let factory = move |seed: u64| make_factory(&black_spec, &white_spec, seed);
    let result = runner.run(factory).map_err(|e| anyhow::anyhow!(e))?;

    // 進捗バーを終了
    if let Some(bar) = progress_bar {
        bar.finish_with_message("done");
    }

    let elapsed_secs = result.elapsed.as_secs_f64();
    let throughput = if elapsed_secs > 0.0 {
        result.num_games as f64 / elapsed_secs
    } else {
        0.0
    };
    let avg_moves = if result.num_games > 0 {
        result.total_moves as f64 / result.num_games as f64
    } else {
        0.0
    };
    let total = result.num_games as f64;
    let pct = |x: usize| -> f64 {
        if total > 0.0 {
            x as f64 * 100.0 / total
        } else {
            0.0
        }
    };

    println!("=== Self-Play Result ===");
    println!("Games:        {}", result.num_games);
    println!("Threads:      {}", args.threads);
    println!("Elapsed:      {:.2} s", elapsed_secs);
    println!("Throughput:   {:.1} games/s", throughput);
    println!(
        "Black wins:   {} ({:.1}%)",
        result.black_wins,
        pct(result.black_wins)
    );
    println!(
        "White wins:   {} ({:.1}%)",
        result.white_wins,
        pct(result.white_wins)
    );
    println!("Draws:        {} ({:.1}%)", result.draws, pct(result.draws));
    println!("Avg moves:    {:.1}", avg_moves);

    if let Some(dir) = &log_dir {
        println!("Log dir:      {}", dir.display());
    }
    if let Some(path) = &args.jsonl_log {
        println!("JSONL log:    {}", path.display());
    }

    Ok(())
}

/// Factory that turns SPEC plus seed into `(black_player, white_player)`.
///
/// The seed for each game is the value supplied by `BatchRunner` XORed with
/// the seed from the SPEC. The `Nn` variant (Phase 6.4) is routed through
/// the `othello-cli` wrapper.
fn make_factory(
    black_spec: &PlayerSpec,
    white_spec: &PlayerSpec,
    seed: u64,
) -> (Box<dyn Player>, Box<dyn Player>) {
    let black =
        crate::player_spec_with_nn::build_with_seed_override(black_spec, Color::Black, seed)
            .unwrap_or_else(|e| panic!("failed to build black player: {e}"));
    let white = crate::player_spec_with_nn::build_with_seed_override(
        white_spec,
        Color::White,
        seed.wrapping_add(0x9E37_79B9),
    )
    .unwrap_or_else(|e| panic!("failed to build white player: {e}"));
    (black, white)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_4_games() {
        let args = Args {
            board_size: 8,
            num_games: 4,
            threads: 2,
            black: "random:seed=1".into(),
            white: "random:seed=2".into(),
            seed: Some(7),
            log_dir: None,
            jsonl_log: None,
            swap_colors: false,
            save_records: SaveRecordsFormat::Json,
            max_moves: None,
            no_progress: true,
        };
        run(args).unwrap();
    }
}
