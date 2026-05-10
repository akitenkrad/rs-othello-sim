[English](../getting-started.md) | [日本語](getting-started.md)

# はじめに

このページは，リポジトリを clone してから 5 分以内に最初の対局を終えるまでをガイドします．読み終える頃には，ワークスペースをビルドし，シミュレーション対局を 1 局走らせ，その棋譜を保存し，TUI で再生できるようになっています．

## 前提条件

- **Rust** 1.85 以上 ( edition 2024 ) ．`cargo` が `PATH` に通っていること．
- **macOS** または **Linux** ．Windows は best-effort 対応．
- *( オプション )* **Python** 3.11 以上と [`uv`](https://docs.astral.sh/uv/) ． `tools/` 配下の解析・可視化ツールに必要です．
- *( オプション )* [`maturin`](https://www.maturin.rs/) ． `crates/othello-py` の PyO3 バインディングを使う場合に必要です．

Rust ワークスペース単体は Python ツールチェインなしでビルドできます．

## 1. Clone

```bash
git clone https://github.com/akitenkrad/rs-othello-sim
cd rs-othello-sim
```

## 2. Build

```bash
cargo build --release --workspace
```

初回ビルドでは Candle ( `crates/othello-nn` の NN evaluator が利用 ) が取得されるため数分かかることがあります．以降のビルドはインクリメンタルで数秒で完了します．開発中は `--release` を外して `cargo build --workspace` の方が高速です．

## 3. 最初の対局を走らせる

seed 付きの random player と greedy player の対局を 1 局シミュレートします:

```bash
./target/release/othello-cli simulate \
  --black random:seed=42 \
  --white greedy
```

最終盤面と勝者，スコアが表示されるはずです． `--board-size N` ( 4 〜 26 ) で盤面サイズを変更できます．

## 4. 棋譜の保存と再生

対局を JSON で保存し， TUI で再生してみます:

```bash
./target/release/othello-cli simulate \
  --black random:seed=42 \
  --white greedy \
  --save-record game.json \
  --record-format json

./target/release/othello-cli replay --file game.json --format json
```

Replay モードでは矢印キーで手順を進められます． `q` で終了．キー一覧は [TUI ガイド](tui-guide.md) を参照してください．

## 5. TUI を直接試す

TUI で 2 人対局:

```bash
./target/release/othello-cli play --board-size 8
```

Evaluator overlay 付きで AI 同士の対局を観戦:

```bash
./target/release/othello-cli observe \
  --black mcts:500 --white greedy --auto-delay 500
```

## 次のステップ

- [CLI 利用ガイド](cli-usage.md) — 全サブコマンドと全フラグのコピペで使えるレシピ集．
- [TUI ガイド](tui-guide.md) — Play / Replay / Observe の各モードと Evaluator overlay．
- [Self-play & バッチ実行](self-play.md) — 進捗バーと構造化ログ付きで数千局を回す方法．
- [Python バインディング & RL](python-rl.md) — `othello-py` 経由の Gymnasium / PettingZoo 環境．
- [アーキテクチャ](architecture.md) — クレートがどう組み合わさっているか．
