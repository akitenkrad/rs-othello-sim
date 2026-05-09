//! `selfplay` サブコマンド: バッチ self-play 実行 ( BatchRunner)．

use anyhow::{Context, Result};
use chrono::Local;
use clap::{Args as ClapArgs, ValueEnum};
use othello_core::{BoardSize, Color};
use othello_engine::{BatchConfig, BatchRunner};
use othello_player::Player;
use othello_player::player_spec::{PlayerSpec, parse_player_spec};
use std::path::PathBuf;

/// 棋譜出力フォーマット ( selfplay)．現状は JSON のみサポート．
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SaveRecordsFormat {
    /// 自前 JSON 形式 ( 1 局 1 ファイル)．
    Json,
}

/// `othello-cli selfplay` の引数．
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// 盤面サイズ．
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,

    /// 試合数．
    #[arg(long, default_value_t = 100)]
    pub num_games: usize,

    /// 並列スレッド数 ( 0 = 論理コア数)．
    #[arg(long, default_value_t = 0)]
    pub threads: usize,

    /// 黒プレイヤー SPEC ( 例 `random:seed=1`，`greedy`，`mcts:200`)．
    #[arg(long, default_value = "random")]
    pub black: String,

    /// 白プレイヤー SPEC．
    #[arg(long, default_value = "random")]
    pub white: String,

    /// 乱数 seed ( 各局には `seed + game_index` が渡る)．
    #[arg(long)]
    pub seed: Option<u64>,

    /// 各ゲームの JSON 棋譜を保存するディレクトリ．
    /// `auto` を指定すると `runs/selfplay_YYYYMMDD_HHMMSS/` に自動生成する．
    #[arg(long)]
    pub log_dir: Option<String>,

    /// 全局を集約する JSONL ログのパス．
    #[arg(long)]
    pub jsonl_log: Option<PathBuf>,

    /// 黒白を偶奇で入れ替える．
    #[arg(long, default_value_t = false)]
    pub swap_colors: bool,

    /// 棋譜の保存フォーマット ( `--log-dir` 指定時のみ意味あり)．
    #[arg(long, value_enum, default_value_t = SaveRecordsFormat::Json)]
    pub save_records: SaveRecordsFormat,

    /// 安全装置 ( この手数を超えたらエラー停止)．
    #[arg(long)]
    pub max_moves: Option<u32>,
}

/// `selfplay` 実行関数．
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

    let cfg = BatchConfig {
        num_games: args.num_games,
        num_threads: args.threads,
        seed: args.seed,
        log_dir: log_dir.clone(),
        swap_colors: args.swap_colors,
        board_size,
        max_moves: args.max_moves,
        jsonl_log_path: args.jsonl_log.clone(),
    };

    eprintln!(
        "Self-play: board={}x{} games={} threads={} swap_colors={}",
        args.board_size, args.board_size, args.num_games, args.threads, args.swap_colors
    );

    let runner = BatchRunner::new(cfg);
    let factory = move |seed: u64| make_factory(&black_spec, &white_spec, seed);
    let result = runner.run(factory).map_err(|e| anyhow::anyhow!(e))?;

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

/// SPEC + seed から `(black_player, white_player)` を生成する factory．
///
/// 各ゲームの seed は `BatchRunner` から渡される値に，SPEC の seed を XOR して撹拌する．
fn make_factory(
    black_spec: &PlayerSpec,
    white_spec: &PlayerSpec,
    seed: u64,
) -> (Box<dyn Player>, Box<dyn Player>) {
    let black = build_with_seed_override(black_spec, Color::Black, seed);
    let white = build_with_seed_override(white_spec, Color::White, seed.wrapping_add(0x9E37_79B9));
    (black, white)
}

fn build_with_seed_override(spec: &PlayerSpec, color: Color, seed: u64) -> Box<dyn Player> {
    let overridden = match spec.clone() {
        PlayerSpec::Random { seed: s } => PlayerSpec::Random { seed: s ^ seed },
        PlayerSpec::Greedy => PlayerSpec::Greedy,
        PlayerSpec::Mcts {
            simulations,
            exploration,
            seed: s,
            max_rollout_depth,
        } => PlayerSpec::Mcts {
            simulations,
            exploration,
            seed: Some(s.unwrap_or(0) ^ seed),
            max_rollout_depth,
        },
    };
    overridden.build_player(color)
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
        };
        run(args).unwrap();
    }
}
