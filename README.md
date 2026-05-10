[English](README.md) | [日本語](README_ja.md)

<h1 align="center">rs-othello-sim</h1>

<p align="center">
  <em>A Rust framework for Othello (Reversi) simulation and reinforcement-learning research.</em>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust&logoColor=white" alt="Rust 1.85+"></a>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/Python-3.11%2B-3776AB?logo=python&logoColor=white" alt="Python 3.11+"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/tests-358%20passing-brightgreen" alt="358 tests passing">
</p>

<p align="center">
  <img src="docs/assets/demo.gif" alt="rs-othello-sim TUI demo" width="780">
</p>

---

## Highlights

- **9-crate Cargo workspace** — `core`, `player`, `io`, `engine`, `rl`, `tui`, `cli`, `py`, `nn`.
- **Hybrid board representation** — 8×8 bitboard for speed, generic `4×4`–`26×26` boards for variety.
- **Players** — Random, Greedy, Human, MCTS (with tree reuse), external engine (Edax / Egaroucid), and a Candle-backed NN evaluator (safetensors / ONNX).
- **Record I/O** — JSON, GGF, WTHOR (read), JSONL event logs; `othello-cli fetch` downloads WTHOR archives directly from the FFO website.
- **TUI** — Play, Replay (with auto-play and adjustable tempo), and Observe (with an MCTS / NN evaluator overlay).
- **RL** — Gymnasium / PettingZoo-compatible environments, plus a uniform / Prioritized Experience Replay buffer.
- **Python** — PyO3 bindings, plus a `tools/` Python toolchain for visualization, analysis, and TensorBoard export.

## Quick install

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
cargo build --release --workspace

# Optional: Python bindings via maturin
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## Quick start

```bash
# One scripted game
./target/release/othello-cli simulate --black random:seed=42 --white greedy

# Watch AIs play in the TUI (Observe mode)
./target/release/othello-cli observe --black mcts:500 --white greedy --auto-delay 500

# Replay a saved record
./target/release/othello-cli replay --file game.json --auto --auto-delay 250

# Fetch the WTHOR 2023 archive from the French Othello Federation
./target/release/othello-cli fetch wthor --year 2023 --dest data/wthor/
```

## Documentation

| Topic | Page |
|---|---|
| Quick walkthrough | [docs/getting-started.md](docs/getting-started.md) |
| Every CLI subcommand | [docs/cli-usage.md](docs/cli-usage.md) |
| TUI keymap and layouts | [docs/tui-guide.md](docs/tui-guide.md) |
| Self-play and batch runs | [docs/self-play.md](docs/self-play.md) |
| Game-record formats | [docs/record-formats.md](docs/record-formats.md) |
| External engines (Edax / Egaroucid) | [docs/external-engines.md](docs/external-engines.md) |
| External data sources (WTHOR / GGF / online) | [docs/external-data.md](docs/external-data.md) |
| Python bindings & RL | [docs/python-rl.md](docs/python-rl.md) |
| Visualization & analysis tools | [docs/tools-visualize.md](docs/tools-visualize.md) |
| NN evaluator | [docs/nn-evaluator.md](docs/nn-evaluator.md) |
| Replay buffer | [docs/replay-buffer.md](docs/replay-buffer.md) |
| Architecture overview | [docs/architecture.md](docs/architecture.md) |
| Benchmark targets and results | [docs/benchmarks.md](docs/benchmarks.md) |

## License

MIT — see [LICENSE](LICENSE).
