//! `play` subcommand: two-player match over stdin/stdout.

use crate::commands::{format_move, render_board};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use othello_core::{BoardSize, GameState, Move};
use othello_player::human::parse_coord;
use std::io::{BufRead, BufReader, Write};

/// Arguments for `othello-cli play`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Board size (4..=26).
    #[arg(long, default_value_t = 8)]
    pub board_size: u8,
}

/// Entry point for `play`. Drives a two-player match interactively via stdin.
pub fn run(args: Args) -> Result<()> {
    let size = BoardSize::square(args.board_size);
    let mut state =
        GameState::standard(size).with_context(|| format!("invalid board size: {size:?}"))?;

    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout();

    while !state.is_terminal() {
        let _ = writeln!(stdout, "{}", render_board(&state));
        let legal = state.legal_moves();
        if legal.is_empty() {
            let _ = writeln!(
                stdout,
                "{:?} has no legal moves; passing.",
                state.side_to_move
            );
            state.apply_move(Move::Pass)?;
            continue;
        }

        let legal_str = legal
            .iter()
            .map(|m| format_move(*m))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(
            stdout,
            "{:?} to move (legal: {legal_str}). Enter move (e.g. d3, pass): ",
            state.side_to_move
        );
        let _ = stdout.flush();

        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            anyhow::bail!("input exhausted");
        }
        let trimmed = line.trim();
        let mv = if trimmed.eq_ignore_ascii_case("pass") {
            Move::Pass
        } else {
            match parse_coord(trimmed) {
                Ok(c) => Move::Place(c),
                Err(e) => {
                    let _ = writeln!(stdout, "Invalid input: {e}. Try again.");
                    continue;
                }
            }
        };

        if let Err(e) = state.apply_move(mv) {
            let _ = writeln!(stdout, "Illegal move: {e}. Try again.");
            continue;
        }
    }

    let _ = writeln!(stdout, "{}", render_board(&state));
    let result = state.result().expect("terminal state must have result");
    let _ = writeln!(
        stdout,
        "Game over. Black={}, White={}, winner={:?}",
        result.black, result.white, result.winner
    );

    Ok(())
}
