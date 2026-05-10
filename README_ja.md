[English](README.md) | [日本語](README_ja.md)

# rs-othello-sim

[![CI](https://img.shields.io/badge/CI-passing-brightgreen)](#)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

オセロ ( リバーシ ) のシミュレーション，強化学習，Engine 同士の対戦実験のための Rust フレームワークです．

![Observe-mode demo](docs/assets/tui-observe.gif)

## クイックインストール

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
cargo build --release --workspace

# Python バインディングを使う場合 ( オプション ):
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## クイックスタート

```bash
./target/release/othello-cli simulate --black random:seed=42 --white greedy
./target/release/othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

## ドキュメント

- [はじめに](docs/ja/getting-started.md)
- [CLI 利用ガイド](docs/ja/cli-usage.md)
- [TUI ガイド](docs/ja/tui-guide.md)
- [Self-play & バッチ実行](docs/ja/self-play.md)
- [レコードフォーマット](docs/ja/record-formats.md)
- [外部データソース](docs/ja/external-data.md)
- [外部 Engine](docs/ja/external-engines.md)
- [Python バインディング & RL](docs/ja/python-rl.md)
- [ツール ( visualize / analyze / TB )](docs/ja/tools-visualize.md)
- [NN Evaluator](docs/ja/nn-evaluator.md)
- [Replay buffer](docs/ja/replay-buffer.md)
- [アーキテクチャ](docs/ja/architecture.md)
- [ベンチマーク](docs/ja/benchmarks.md)

## ライセンス

MIT
