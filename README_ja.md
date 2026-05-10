[English](README.md) | [日本語](README_ja.md)

<h1 align="center">rs-othello-sim</h1>

<p align="center">
  <em>オセロ ( Reversi) のシミュレーションと強化学習研究のための Rust フレームワーク．</em>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust&logoColor=white" alt="Rust 1.85+"></a>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/Python-3.11%2B-3776AB?logo=python&logoColor=white" alt="Python 3.11+"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/tests-358%20passing-brightgreen" alt="358 tests passing">
</p>

<p align="center">
  <img src="docs/assets/demo.gif" alt="rs-othello-sim TUI のデモ" width="780">
</p>

---

## 主な機能

- **9 クレート Cargo workspace** — `core` / `player` / `io` / `engine` / `rl` / `tui` / `cli` / `py` / `nn`．
- **ハイブリッド盤面表現** — 8×8 は高速 bitboard，4×4 〜 26×26 は汎用盤．
- **プレイヤー** — Random / Greedy / Human / MCTS ( tree reuse 対応) / 外部エンジン ( Edax / Egaroucid) / Candle ベースの NN evaluator ( safetensors / ONNX)．
- **棋譜 I/O** — JSON / GGF / WTHOR ( 読込) / JSONL イベントログ．`othello-cli fetch` で FFO 公式サイトから WTHOR を直接取得．
- **TUI** — Play，Replay ( 自動再生 + テンポ調整)，Observe ( MCTS / NN evaluator オーバーレイ付き)．
- **RL** — Gymnasium / PettingZoo 互換環境と Uniform / PER ( Prioritized Experience Replay) buffer．
- **Python** — PyO3 バインディングに加え， `tools/` 配下に可視化 / 解析 / TensorBoard 連携の Python ツールチェーン．

## クイックインストール

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
cargo build --release --workspace

# Python バインディングを使う場合 ( 任意)
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## クイックスタート

```bash
# 1 局シミュレーション
./target/release/othello-cli simulate --black random:seed=42 --white greedy

# TUI の Observe モードで AI 同士の対局を観戦
./target/release/othello-cli observe --black mcts:500 --white greedy --auto-delay 500

# 保存済み棋譜の自動再生
./target/release/othello-cli replay --file game.json --auto --auto-delay 250

# フランスオセロ連盟 ( FFO) の WTHOR 2023 アーカイブを取得
./target/release/othello-cli fetch wthor --year 2023 --dest data/wthor/
```

## ドキュメント

| トピック | ページ |
|---|---|
| 最小例とウォークスルー | [docs/ja/getting-started.md](docs/ja/getting-started.md) |
| 全 CLI サブコマンド | [docs/ja/cli-usage.md](docs/ja/cli-usage.md) |
| TUI のキー操作と画面構成 | [docs/ja/tui-guide.md](docs/ja/tui-guide.md) |
| Self-play とバッチ運用 | [docs/ja/self-play.md](docs/ja/self-play.md) |
| 棋譜フォーマット | [docs/ja/record-formats.md](docs/ja/record-formats.md) |
| 外部エンジン ( Edax / Egaroucid) | [docs/ja/external-engines.md](docs/ja/external-engines.md) |
| 外部データソース ( WTHOR / GGF / オンライン) | [docs/ja/external-data.md](docs/ja/external-data.md) |
| Python バインディング & RL | [docs/ja/python-rl.md](docs/ja/python-rl.md) |
| 可視化・分析ツール | [docs/ja/tools-visualize.md](docs/ja/tools-visualize.md) |
| NN evaluator | [docs/ja/nn-evaluator.md](docs/ja/nn-evaluator.md) |
| Replay buffer | [docs/ja/replay-buffer.md](docs/ja/replay-buffer.md) |
| アーキテクチャ概観 | [docs/ja/architecture.md](docs/ja/architecture.md) |
| ベンチマーク目標と結果 | [docs/ja/benchmarks.md](docs/ja/benchmarks.md) |

## ライセンス

MIT — [LICENSE](LICENSE) を参照．
