# rs-othello-sim

A Rust-based integrated framework for Othello (Reversi) simulation research and reinforcement-learning experiments.

## Overview

`rs-othello-sim` is a workspace that incrementally provides a variable-size Othello game core, pluggable player strategies, game-record I/O, a TUI/CLI, RL environments, Python bindings, external-engine integration, and a Python tool chain for analysis and visualization. Othello — being a fully observable, deterministic, two-player zero-sum game — serves as a controllable testbed for RL algorithm validation and Theory-of-Mind research.

The full design document lives in the parent Obsidian vault at `設計書/Othello_シミュレータ設計書.md`.

## Implementation Status

The implementation is staged across five phases (see design document §10).

| Phase | Scope | Status |
|---|---|---|
| Phase 1 | Core game (`othello-core`) | Done |
| Phase 2 | Game loop layer (`othello-player`, `othello-engine`, `othello-io`, `othello-cli`) | Done |
| Phase 3 | TUI, JSONL logger, WTHOR reader, `convert` / `inspect` subcommands | Done |
| Phase 4 | RL environments (`othello-rl`), Python bindings (`othello-py`), `selfplay` batch runner, `BatchRunner`, MCTS player, TUI Observe mode | Done |
| Phase 5 | External engine integration, `tools/` Python visualization / analysis / TensorBoard converter, indicatif progress bars, Evaluator overlay | Done |
| Phase 6 (in progress) | 6.1 real-engine smoke test, 6.2 MCTS tree reuse, 6.4 Candle-based NN evaluator | In progress |

## Crates

```
crates/
├── othello-core/        # Board, rules, game state (Bitboard8 + GenericBoard hybrid)
├── othello-player/      # Player trait + Random / Greedy / Human / MCTS / ExternalEngine + Evaluator trait
├── othello-io/          # GameRecord neutral representation + JSON / GGF / WTHOR readers and writers
├── othello-engine/      # GameEngine, full-snapshot GameHistory, Replayer, BatchRunner (rayon, ProgressCallback)
├── othello-rl/          # Gymnasium / PettingZoo compatible environments
├── othello-tui/         # ratatui frontend (Play / Replay / Observe modes; Evaluator overlay in Observe)
├── othello-nn/          # Candle policy/value evaluator (Phase 6.4)
├── othello-cli/         # `othello-cli` binary (play / simulate / replay / convert / inspect / selfplay / benchmark / observe)
└── othello-py/          # PyO3 bindings (`maturin develop` to install)
```

```
tools/                   # uv workspace (Python toolchain)
├── visualize/           # Game-record frame renderer (matplotlib) + learning-curve plotter
├── analyze/             # Statistical aggregation / two-run comparison (pandas)
└── tb_converter/        # JSONL → TensorBoard event-file converter (tensorboardX)
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

### `othello-player` — Phase 5 additions

| Type | Role |
|---|---|
| `Evaluator` | Per-move score map (e.g. MCTS visit counts), used by TUI Observe overlay |
| `ExternalEnginePlayer` | Synchronous-IO subprocess driver |
| `ExternalEngineConfig` | Command path, args, protocol, timeout |
| `Protocol` | `Gtp` (default) / `Ntest` (Edax / Egaroucid-style) |

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

# Generate API docs (warnings shown if doc comments are missing)
cargo doc --workspace --no-deps
```

### Python tools (`uv` required)

```bash
uv sync --all-packages
uv run pytest

# Or explicitly:
uv run pytest tools/visualize/tests/
uv run pytest tools/analyze/tests/
uv run pytest tools/tb_converter/tests/
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

Player specification grammar (design §5.3):

```
random[:seed=N]
greedy
mcts:N[,c=F][,seed=M][,depth=D]
external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,arg=VAL,...]
nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
```

The `nn:` variants load a Candle policy/value model (Phase 6.4) and use it as
both `Player` and `Evaluator`. See [`docs/nn-evaluator.md`](docs/nn-evaluator.md)
for the IO schema, supported weight formats, and CLI examples.

### Self-play batch runner

```bash
cargo run --release -p othello-cli -- selfplay \
  --board-size 8 \
  --num-games 1000 \
  --threads 8 \
  --black mcts:200 \
  --white random:seed=1 \
  --seed 42 \
  --log-dir auto \
  --swap-colors
```

`--log-dir auto` writes per-game JSON records to `runs/selfplay_YYYYMMDD_HHMMSS/`.
A live `indicatif` progress bar is shown by default; pass `--no-progress` to suppress it.
Pass `--log-file path.json` (global flag) to also dump structured logs to a file.

### External engine integration

```bash
# GTP-style external engine
cargo run -p othello-cli -- selfplay \
  --black "external:./engines/egaroucid,protocol=gtp,arg=--level,arg=1" \
  --white "greedy" \
  --num-games 10

# Edax-style ntest protocol
cargo run -p othello-cli -- observe \
  --black "external:/usr/local/bin/edax,protocol=ntest,timeout=10" \
  --white "mcts:500"
```

Note: under high `--threads` × `--num-games` settings, every game spawns its own engine
subprocess. Be mindful of process limits and per-engine memory footprint.

Optional: real engine binaries (Edax / Egaroucid) can be fetched via `scripts/fetch_engines.sh`
for local smoke testing. See [`docs/external-engines.md`](docs/external-engines.md) for setup,
license caveats, and the `cargo test --test real_engine -- --ignored` workflow.

### Benchmark

```bash
cargo run --release -p othello-cli -- benchmark --target legal-moves --duration 5
cargo run --release -p othello-cli -- benchmark --target self-play --board-size 8 --duration 5
```

### Observe (TUI for AI vs AI)

```bash
cargo run -p othello-cli -- observe --board-size 8 --black mcts:500 --white greedy --auto-delay 500
```

When the side-to-move's player implements `Evaluator` (e.g. `MctsPlayer`), an
**Evaluator overlay** panel shows the top legal moves with a small bar chart of
their normalized visit counts. The overlay updates only after each `select_move`
call (no intermediate updates while the engine is searching).

### Python bindings

See [`crates/othello-py/README.md`](crates/othello-py/README.md). Build with:

```bash
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

Then in Python:

```python
import othello_sim
env = othello_sim.OthelloEnv(board_size=8, opponent="mcts:200", reward_mode="sparse", seed=42)
obs, info = env.reset()
obs, reward, terminated, truncated, info = env.step(int(info["action_mask"].nonzero()[0][0]))
```

### Python tools (analysis & visualization)

```bash
# Render every move of a JSON game record as PNG frames or an animated GIF
uv run plot-game --input game.json --output-dir frames/
uv run plot-game --input game.json --gif game.gif

# Win-rate / move-count curves from JSONL logs
uv run plot-curve --input runs/selfplay_*.jsonl --output curve.png

# Aggregate statistics
uv run analyze-stats --input runs/selfplay_20260509/ --output stats.csv

# Two-run comparison with chi-square test
uv run analyze-compare --a runs/run_a --b runs/run_b

# JSONL → TensorBoard
uv run jsonl-to-tb --input runs/selfplay_*.jsonl --output runs/tb/
tensorboard --logdir runs/tb/
```

## Design Highlights

- **Hybrid board representation**: 8×8 uses `Bitboard8` (`u64 × 2`) for speed; 4×4 to 26×26 uses `GenericBoard` (`Vec<Option<Color>>`).
- **Bitboard layout**: `bit_index = row * 8 + col` (row 0 col 0 → bit 0).
- **8-direction shifts**: column masks (A and H files) prevent wrap-around; chained-stone masks are built with five shift–OR iterations.
- **Equivalence guarantee**: property-based tests verify that `Bitboard8` and `GenericBoard` produce identical legal-move sets and flipped-stone sets on 8×8.
- **Full-snapshot history**: each move pushes a complete `GameState` snapshot, making `step_forward` / `step_backward` / `jump_to(n)` $O(1)$.
- **Pass handling**: when the side to move has no legal moves, the engine auto-passes; players returning `Pass` while legal moves exist trigger an error.
- **External-engine isolation**: synchronous IO (`std::process::Command`) per `ExternalEnginePlayer`, with a soft per-move timeout enforced via a worker thread + `mpsc::channel`.

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
| Integration | End-to-end game runs, record round-trips, replayer consistency, external-engine mock dialogues |
| Property (`proptest`) | Termination of random play, `Bitboard8` ⇔ `GenericBoard` equivalence, JSON round-trip idempotence |
| Snapshot (`insta`) | TUI rendering for Play / Replay / Observe / Evaluator-overlay views |
| Python | `pytest` smoke tests for visualization, statistics, and TensorBoard conversion (Unix only for external mocks) |

## Repository Layout

```
rs-othello-sim/
├── Cargo.toml         # Workspace manifest with shared dependencies
├── Cargo.lock
├── pyproject.toml     # uv workspace root for tools/
├── CHANGELOG.md       # Phase-by-phase change history
├── CLAUDE.md          # Internal Claude Code guidance (Japanese)
├── README.md          # This file
├── .cargo/config.toml # PYO3_USE_ABI3_FORWARD_COMPATIBILITY for Python 3.14+ environments
├── crates/
│   ├── othello-core/
│   ├── othello-player/
│   │   └── src/external/   # ExternalEnginePlayer + GtpProtocol + NtestProtocol
│   ├── othello-io/
│   ├── othello-engine/
│   ├── othello-rl/
│   ├── othello-tui/
│   ├── othello-cli/
│   └── othello-py/
└── tools/
    ├── visualize/
    ├── analyze/
    └── tb_converter/
```

## Toolchain

- Rust edition `2024`, `rust-version = "1.85"`
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` kept clean
- `unsafe` is not used
- Python `>= 3.11`, `ruff` + `pytest` for the `tools/` workspace

## License

MIT
