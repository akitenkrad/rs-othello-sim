//! `benchmark` subcommand: simple hot-loop measurements (a lightweight alternative to criterion).

use anyhow::{Result, bail};
use clap::{Args as ClapArgs, ValueEnum};
use othello_core::{BoardSize, Color, GameState, Move};
use othello_player::{Player, RandomPlayer};
use std::time::{Duration, Instant};

/// Benchmark target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// Legal-move generation throughput (auto-selects bitboard or generic).
    LegalMoves,
    /// Per-move throughput for Random vs Random self-play.
    SelfPlay,
}

/// Arguments for `othello-cli benchmark`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Benchmark target.
    #[arg(long, value_enum)]
    pub target: Target,
    /// Measurement duration in seconds.
    #[arg(long, default_value_t = 5.0)]
    pub duration: f64,
    /// Board size (used for self-play).
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,
}

/// Entry point for `benchmark`.
pub fn run(args: Args) -> Result<()> {
    if args.duration <= 0.0 {
        bail!("--duration must be > 0");
    }
    let dur = Duration::from_secs_f64(args.duration);
    match args.target {
        Target::LegalMoves => bench_legal_moves(dur, args.board_size)?,
        Target::SelfPlay => bench_self_play(dur, args.board_size)?,
    }
    Ok(())
}

fn bench_legal_moves(dur: Duration, board_size: u8) -> Result<()> {
    let state = GameState::standard(BoardSize::square(board_size))
        .map_err(|e| anyhow::anyhow!("invalid board size: {e}"))?;
    let start = Instant::now();
    let mut iters: u64 = 0;
    while start.elapsed() < dur {
        // 16 回ずつ計測してオーバーヘッドを薄める
        for _ in 0..16 {
            let _ = state.legal_moves();
            iters += 1;
        }
    }
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let throughput = if secs > 0.0 { iters as f64 / secs } else { 0.0 };
    println!(
        "Target: legal-moves ({}x{} {})",
        board_size,
        board_size,
        if board_size == 8 {
            "Bitboard"
        } else {
            "Generic"
        }
    );
    println!("Duration: {:.2} s", secs);
    println!("Iterations: {}", format_with_underscores(iters));
    println!(
        "Throughput: {} ops/s",
        format_with_underscores(throughput as u64)
    );
    Ok(())
}

fn bench_self_play(dur: Duration, board_size: u8) -> Result<()> {
    let size = BoardSize::square(board_size);
    let mut total_moves: u64 = 0;
    let mut total_games: u64 = 0;
    let start = Instant::now();
    let mut seed: u64 = 0;
    while start.elapsed() < dur {
        let mut s = GameState::standard(size).map_err(|e| anyhow::anyhow!(e))?;
        let mut black = RandomPlayer::with_seed(Color::Black, seed);
        let mut white = RandomPlayer::with_seed(Color::White, seed.wrapping_add(1));
        seed = seed.wrapping_add(2);
        let mut moves: u64 = 0;
        let mut safety = 1024u32;
        while !s.is_terminal() && safety > 0 {
            let mv = if s.legal_moves().is_empty() {
                Move::Pass
            } else if s.side_to_move == Color::Black {
                black.select_move(&s).map_err(|e| anyhow::anyhow!(e))?
            } else {
                white.select_move(&s).map_err(|e| anyhow::anyhow!(e))?
            };
            s.apply_move(mv).map_err(|e| anyhow::anyhow!(e))?;
            moves += 1;
            safety -= 1;
        }
        total_moves += moves;
        total_games += 1;
    }
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let mps = if secs > 0.0 {
        total_moves as f64 / secs
    } else {
        0.0
    };
    let gps = if secs > 0.0 {
        total_games as f64 / secs
    } else {
        0.0
    };
    println!("Target: self-play ({}x{})", board_size, board_size);
    println!("Duration: {:.2} s", secs);
    println!("Games: {}", format_with_underscores(total_games));
    println!("Moves: {}", format_with_underscores(total_moves));
    println!(
        "Throughput: {} moves/s",
        format_with_underscores(mps as u64)
    );
    println!("Games/s: {:.1}", gps);
    Ok(())
}

/// Format a number with `_` as a thousands separator (e.g. 12345678 -> "12_345_678").
fn format_with_underscores(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        let from_end = bytes.len() - i;
        if i > 0 && from_end % 3 == 0 {
            out.push('_');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_moves_short_run() {
        let args = Args {
            target: Target::LegalMoves,
            duration: 0.05,
            board_size: 8,
        };
        run(args).unwrap();
    }

    #[test]
    fn self_play_short_run() {
        let args = Args {
            target: Target::SelfPlay,
            duration: 0.05,
            board_size: 8,
        };
        run(args).unwrap();
    }

    #[test]
    fn rejects_zero_duration() {
        let args = Args {
            target: Target::LegalMoves,
            duration: 0.0,
            board_size: 8,
        };
        assert!(run(args).is_err());
    }

    #[test]
    fn underscore_formatter() {
        assert_eq!(format_with_underscores(0), "0");
        assert_eq!(format_with_underscores(123), "123");
        assert_eq!(format_with_underscores(1234), "1_234");
        assert_eq!(format_with_underscores(1234567), "1_234_567");
        assert_eq!(format_with_underscores(12345678), "12_345_678");
    }
}
