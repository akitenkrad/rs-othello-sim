# rs-othello-sim

[![CI](https://img.shields.io/badge/CI-passing-brightgreen)](#)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

A Rust framework for Othello (Reversi) simulation, reinforcement
learning, and engine-vs-engine experiments.

![Observe-mode demo](docs/assets/tui-observe.gif)

## Quick install

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
cargo build --release --workspace

# Optional Python bindings:
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## Quick start

```bash
./target/release/othello-cli simulate --black random:seed=42 --white greedy
./target/release/othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

## Documentation

- [Getting started](docs/getting-started.md)
- [CLI usage](docs/cli-usage.md)
- [TUI guide](docs/tui-guide.md)
- [Self-play & batch runs](docs/self-play.md)
- [Record formats](docs/record-formats.md)
- [External engines](docs/external-engines.md)
- [Python bindings & RL](docs/python-rl.md)
- [Tools (visualize / analyze / TB)](docs/tools-visualize.md)
- [NN evaluator](docs/nn-evaluator.md)
- [Replay buffer](docs/replay-buffer.md)
- [Architecture](docs/architecture.md)
- [Benchmarks](docs/benchmarks.md)
- [Contributing](docs/contributing.md)

## License

MIT
