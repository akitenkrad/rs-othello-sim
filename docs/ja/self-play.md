[English](../self-play.md) | [日本語](self-play.md)

# Self-play & バッチ実行

`othello-cli selfplay` は `othello-engine::BatchRunner` ( Rayon ラッパー ) を使って多数の対局を並列に実行します．戦略のベンチマーク，ニューラルネット evaluator のための学習データ生成，および「N 局回して集計する」系のあらゆるワークフローに使えます．

## クイックスタート

```bash
othello-cli selfplay \
  --num-games 1000 \
  --threads 8 \
  --black mcts:200 \
  --white random:seed=1 \
  --seed 42 \
  --log-dir auto \
  --swap-colors
```

| フラグ | デフォルト | 説明 |
|---|---|---|
| `--num-games N` | 100 | 実行する総対局数 |
| `--threads N` | 0 ( auto ) | 0 は「論理コア数」を意味する |
| `--black SPEC` / `--white SPEC` | `random` | Player spec ( [cli-usage.md](cli-usage.md#playerspec-文法) を参照 ) |
| `--seed N` | _なし_ | 各局の seed は `seed + game_index` |
| `--log-dir DIR` または `auto` | _なし_ | 出力先ディレクトリ． `auto` は `runs/selfplay_YYYYMMDD_HHMMSS/` を選ぶ |
| `--jsonl-log PATH` | _なし_ | 集約 JSONL ログのパス ( 多くの場合 `runs/.../all.jsonl` ) |
| `--save-records FORMAT` | `json` | `--log-dir` 配下の局ごとの棋譜フォーマット |
| `--swap-colors` | off | 奇数番号の局で Black / White を入れ替える |
| `--max-moves N` | _なし_ | 安全装置．N ply を超えたら abort |
| `--no-progress` | off | `indicatif` 進捗バーを抑制 |

## 推奨ディレクトリレイアウト

`--log-dir auto` は自己記述的なツリーを生成します:

```
runs/
└── selfplay_20260510_120000/
    ├── game_0000.json
    ├── game_0001.json
    ├── …
    └── all.jsonl          # 集約 JSONL ログ
```

`game_*.json` は完全な `GameRecord` ( [record-formats.md](record-formats.md) 参照 ) で， `all.jsonl` は `othello-io::JsonlLogger` が出力する JSONL イベントストリームです．どちらも [tools-visualize.md](tools-visualize.md) の Python ツールチェインで処理できます．

## `--swap-colors`

オセロでは色が結果に影響します．多くのヒューリスティック Player は White を持つと不利になりがちです． `--swap-colors` は，同じ戦略ペアを 2 度 ( Black 役と White 役を入れ替えて ) 走らせることで，集計勝率を対称にします:

- 局 0, 2, 4, …: `--black SPEC_A` ， `--white SPEC_B`
- 局 1, 3, 5, …: `--black SPEC_B` ， `--white SPEC_A`

半分の対局を一方の配置で，もう半分を逆の配置で行うことで，色の非対称性によるノイズが head-to-head の指標から取り除かれます．

## 進捗バー

stderr が TTY のとき， `indicatif::ProgressBar` が局単位の進捗メータを表示します:

```
[00:00:42] [#######################-----------] 723/1000 (72%) ETA 16s
```

進捗バーは `BatchConfig.progress` に組み込まれた `ProgressCallback` トレイトを通じて，対局終了ごとに 1 回更新を受け取ります． CLI 以外の連携が必要な場合，ライブラリ利用者はこのトレイトを自前で実装できます．

## 大規模実行時の注意点

- **外部 Engine** ( `external:` SPEC ) は対局ごとにサブプロセスを 1 つ起動します．8 threads × 1000 games なら最大同時 8 Engine プロセス ( 各局が 1 ペア保持 ) になります．Engine の RSS とプロセスごとのファイルディスクリプタ上限を見ておきましょう．OOM killer が反応する前に `ulimit -n` や `ulimit -u` に到達することがあります． [external-engines.md](external-engines.md) を参照してください．
- **NN evaluator** ( `nn:` SPEC ) はスポーンされた Player ごとにモデルをロードします．Candle CPU では小さいながら無視できないコストです．過剰なオーバヘッドを感じたら， `BatchRunner` が `Arc<Model>` を共有できるようになるまで ( 計画中 ) シングルプロセスドライバを優先してください．
- **決定性**．各対局の RNG seed は `seed + game_index` です．完全再現性のあるバッチが必要なら `--seed` を固定し，決定論的な Player spec ( `random:seed=…` ， `mcts:N,seed=M` ， `nn:…,deterministic` ) を使ってください．

## 出力解析

バッチ完了後，解析ツールで集計できます:

```bash
# 局ごとの CSV 統計 ( 勝率，平均手数，序盤分布 )
uv run analyze-stats --input runs/selfplay_20260510_120000/ --output stats.csv

# 2 バッチの比較 ( カイ二乗 / McNemar 検定 )
uv run analyze-compare --a runs/run_a --b runs/run_b

# JSONL → TensorBoard scalars
uv run jsonl-to-tb --input runs/selfplay_20260510_120000/all.jsonl \
                   --output runs/tb/
tensorboard --logdir runs/tb/
```

Python ツールチェイン全体は [tools-visualize.md](tools-visualize.md) を参照してください．

## Self-play 学習ループ ( スケッチ )

`selfplay` は AlphaZero 風学習ループの片側です．もう一方は [Replay buffer](replay-buffer.md) と Trainer ( Python または Rust ) です．組み合わせると以下の流れになります:

```text
loop:
    # 1. 現在のモデルで新しい対局を生成．
    othello-cli selfplay \
        --num-games N \
        --black "nn:safetensors:checkpoints/latest.safetensors" \
        --white "nn:safetensors:checkpoints/latest.safetensors" \
        --log-dir auto

    # 2. 生成された JSON 棋譜を Transition に変換．
    python trainer.py --records runs/selfplay_<ts>/ \
                      --buffer  buffers/per.bin

    # 3. buffer からサンプリングして 1 epoch 以上学習．
    python trainer.py --train --buffer buffers/per.bin \
                              --out checkpoints/next.safetensors

    # 4. checkpoint を昇格させて繰り返す．
    cp checkpoints/next.safetensors checkpoints/latest.safetensors
```

buffer の Python ラッパー ( [replay-buffer.md](replay-buffer.md) を参照 ) は `push` / `sample` / `update_priorities` を numpy 互換で公開しており， `nn:` Player spec が学習結果の重みを消費します．IO スキーマと対応フォーマットは [nn-evaluator.md](nn-evaluator.md) を参照してください．
