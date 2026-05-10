[English](getting-started.md) | [日本語](ja/getting-started.md)

# Getting Started

This page walks you from a fresh clone to a finished game in under five
minutes. By the end you will have built the workspace, played a single
simulated game, saved its record, and replayed it inside the TUI.

## Prerequisites

- **Rust** 1.85+ with `cargo` on `PATH` (Rust edition 2024).
- **macOS** or **Linux**. Windows is best-effort.
- *(Optional)* **Python** 3.11+ and [`uv`](https://docs.astral.sh/uv/) for
  the analysis / visualization tools under `tools/`.
- *(Optional)* [`maturin`](https://www.maturin.rs/) if you want to use
  the PyO3 bindings (`crates/othello-py`).

The Rust workspace builds without any Python toolchain.

## 1. Clone

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
```

## 2. Build

```bash
cargo build --release --workspace
```

The first build pulls Candle (used by the NN evaluator in
`crates/othello-nn`) and may take several minutes; subsequent builds are
incremental and complete in seconds. Run `cargo build --workspace`
without `--release` for a faster debug build during development.

## 3. Play your first game

Run a single simulated game between a seeded random player and a greedy
player:

```bash
./target/release/othello-cli simulate \
  --black random:seed=42 \
  --white greedy
```

You should see a final board, the winner, and the score. Pass
`--board-size N` (4–26) to change the board.

## 4. Save and replay a record

Save the game to JSON and replay it inside the TUI:

```bash
./target/release/othello-cli simulate \
  --black random:seed=42 \
  --white greedy \
  --save-record game.json \
  --record-format json

./target/release/othello-cli replay --file game.json --format json
```

In Replay mode use the arrow keys to step through moves; press `q` to
quit. See [TUI Guide](tui-guide.md) for the full keymap.

## 5. Try the TUI directly

Two-player game over the TUI:

```bash
./target/release/othello-cli play --board-size 8
```

Watch two AIs play with an evaluator overlay:

```bash
./target/release/othello-cli observe \
  --black mcts:500 --white greedy --auto-delay 500
```

## Next steps

- [CLI usage](cli-usage.md) — every subcommand, every flag, with
  copy-paste recipes.
- [TUI guide](tui-guide.md) — Play / Replay / Observe modes and the
  Evaluator overlay.
- [Self-play & batch runs](self-play.md) — running thousands of games
  with progress bars and structured logs.
- [Python bindings & RL](python-rl.md) — Gymnasium / PettingZoo
  environments through `othello-py`.
- [Architecture](architecture.md) — how the crates fit together.
