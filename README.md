# rs-othello-sim

A Rust-based integrated framework for Othello (Reversi) simulation research and reinforcement-learning experiments.

## Overview

`rs-othello-sim` is a workspace that incrementally provides a variable-size Othello game core, pluggable player strategies, game-record I/O, a TUI/CLI, RL environments, and Python bindings. Othello — being a fully observable, deterministic, two-player zero-sum game — serves as a controllable testbed for RL algorithm validation and Theory-of-Mind research.

The full design document lives in the parent Obsidian vault at `設計書/Othello_シミュレータ設計書.md`.

## Implementation Status

The implementation is staged across five phases (see design document §10).

| Phase | Scope | Status |
|---|---|---|
| Phase 1 | Core game (`othello-core`) | Done |
| Phase 2 | Game loop layer (`othello-player`, `othello-engine`, `othello-io`, `othello-cli`) | Done |
| Phase 3 | TUI, JSONL logger, WTHOR reader, `convert` / `inspect` subcommands | Pending |
| Phase 4 | RL environments (`othello-rl`), Python bindings (`othello-py`), `selfplay` batch runner, MCTS player | Pending |
| Phase 5 | External engine integration (Edax / Egaroucid), visualization tools | Pending |

## Crates

```
crates/
├── othello-core/        # Board, rules, game state (Bitboard8 + GenericBoard hybrid)
├── othello-player/      # Player trait + Random / Greedy / Human implementations
├── othello-io/          # GameRecord neutral representation + JSON / GGF readers and writers
├── othello-engine/      # GameEngine, full-snapshot GameHistory, Replayer
└── othello-cli/         # `othello-cli` binary with `play` / `simulate` subcommands
```

### `othello-core` — main types

| Type | Role |
|---|---|
| `Color` | Black / White enum with `opponent()` |
| `Coord` | `(row: u8, col: u8)` board coordinate |
| `Move` | `Place(Coord)` or `Pass` |
| `BoardSize` | Board dimensions (`rows`, `cols`) |
| `Bitboard8` | 8×8 dedicated bitboard (`u64 × 2`) |
| `GenericBoard` | Variable size from 4×4 up to 26×26 |
| `Board` | Hybrid enum dispatching to `Bitboard8` or `Generic` |
| `GameState` | Board + side-to-move + move number + consecutive-pass counter |
| `GameResult` | Winner and final stone counts |
| `OthelloError` | Library error type (`thiserror`-derived) |

## Build & Test

```bash
# Build all crates
cargo build --workspace

# Run all tests (unit + integration + property-based)
cargo test --workspace

# Lint (CI-strict, no warnings)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --check

# Run criterion benchmarks
cargo bench

# Compile-only check for benchmarks
cargo bench --no-run
```

## CLI Quick Start

```bash
# Two-player game over standard I/O (Human vs Human)
cargo run -p othello-cli -- play --board-size 8

# Single simulated game and record export (JSON or GGF)
cargo run -p othello-cli -- simulate \
  --board-size 8 \
  --black random:seed=42 \
  --white greedy \
  --save-record game.json \
  --record-format json
```

Player specification grammar (Phase 2 subset of design §5.3):

```
random[:seed=N]
greedy
```

`mcts:N` and `external:PATH` will be added in Phases 4 and 5 respectively.

## Design Highlights

- **Hybrid board representation**: 8×8 uses `Bitboard8` (`u64 × 2`) for speed; 4×4 to 26×26 uses `GenericBoard` (`Vec<Option<Color>>`).
- **Bitboard layout**: `bit_index = row * 8 + col` (row 0 col 0 → bit 0).
- **8-direction shifts**: column masks (A and H files) prevent wrap-around; chained-stone masks are built with five shift–OR iterations.
- **Equivalence guarantee**: property-based tests verify that `Bitboard8` and `GenericBoard` produce identical legal-move sets and flipped-stone sets on 8×8.
- **Full-snapshot history**: each move pushes a complete `GameState` snapshot, making `step_forward` / `step_backward` / `jump_to(n)` $O(1)$.
- **Pass handling**: when the side to move has no legal moves, the engine auto-passes; players returning `Pass` while legal moves exist trigger an error.

## Benchmark Targets (design document §9)

| Metric | Target |
|---|---|
| 8×8 random self-play | $\geq 10^6$ moves/sec (single thread) |
| 8×8 legal-move generation | $\geq 10^7$ ops/sec (Bitboard) |
| 16×16 random self-play | $\geq 10^4$ moves/sec (Generic) |

Bench measurements have not yet been recorded in CI; only `cargo bench --no-run` (compile check) is verified.

## Testing

| Layer | What is checked |
|---|---|
| Unit | Per-module behaviour for every public type |
| Integration | End-to-end game runs, record round-trips, replayer consistency |
| Property (`proptest`) | Termination of random play, `Bitboard8` ⇔ `GenericBoard` equivalence, JSON round-trip idempotence |

The current test suite passes 110 cases across all crates.

## Repository Layout

```
rs-othello-sim/
├── Cargo.toml         # Workspace manifest with shared dependencies
├── Cargo.lock
├── CLAUDE.md          # Internal Claude Code guidance (Japanese)
├── README.md          # This file
└── crates/
    ├── othello-core/
    ├── othello-player/
    ├── othello-io/
    ├── othello-engine/
    └── othello-cli/
```

Future phases will add `othello-tui/`, `othello-rl/`, and `othello-py/`.

## Toolchain

- Rust edition `2024`, `rust-version = "1.85"`
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` kept clean
- `unsafe` is not used

## License

MIT
