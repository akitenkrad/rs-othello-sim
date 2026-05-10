[English](../tools-visualize.md) | [日本語](tools-visualize.md)

# ツール ( visualize / analyze / TB )

`tools/` 以下に 3 つの小さな Python パッケージがあり，まとめて `uv` ワークスペースを構成します．それぞれコンソールスクリプトとして公開されており， `othello-cli` が生成する棋譜と JSONL ログのポストプロセスを担います．

| パッケージ | コンソールスクリプト | 用途 |
|---|---|---|
| `tools/visualize` | `plot-game` ， `plot-curve` | matplotlib による対局フレーム描画と学習曲線プロット |
| `tools/analyze` | `analyze-stats` ， `analyze-compare` | Pandas ベースの集計と run 比較 |
| `tools/tb_converter` | `jsonl-to-tb` | JSONL → TensorBoard イベントファイル |

`uv sync --all-packages` で 3 パッケージを一括インストールできます．

## セットアップ

```bash
# リポジトリルートで実行
uv sync --all-packages

# 動作確認
uv run plot-game --help
```

## `plot-game` — 単一棋譜のレンダリング

```bash
# ply ごとの PNG をディレクトリに出力
uv run plot-game --input game.json --output-dir frames/

# アニメ GIF ( 1 ply 1 フレーム )
uv run plot-game --input game.json --gif game.gif
```

JSON 棋譜を読み込み，move リストを巡回して各 ply を matplotlib の figure として描画します．盤面レンダラ自体は `visualize.board_render.render_board` として公開されており，notebook で再利用できます． 1 局を視覚的に検証する用途 ( 例: WTHOR から変換した棋譜のサニティチェック ) に便利です．

このツールが消費する棋譜のスキーマは [record-formats.md](record-formats.md) を参照してください．

## `plot-curve` — JSONL からの学習曲線

```bash
uv run plot-curve --input runs/selfplay_*/all.jsonl --output curve.png
```

1 つまたは複数の JSONL ログを集約し，勝率 / 手数曲線をプロットします．入力ファイルそれぞれが 1 本の線になるので，run 間の比較ができます．

## `analyze-stats` — run ディレクトリの集計

```bash
uv run analyze-stats --input runs/selfplay_20260510_120000/ \
                     --output stats.csv
```

レポート内容:

- 勝率 ( Black / White / 引き分け ) ．
- 平均手数．
- 開幕手の分布．
- パス数の分布．

`selfplay --log-dir auto` が出力する局ごとの JSON 棋譜を対象とします．CSV 出力なので Jupyter やスプレッドシートに取り込むのも容易です．

## `analyze-compare` — 2 run の統計比較

```bash
uv run analyze-compare --a runs/run_a --b runs/run_b
```

2 バッチの勝率分布に対しカイ二乗検定と McNemar 検定を実行します．出力にはクロス集計表，検定統計量， p 値，1 行の結論が含まれます．

## `jsonl-to-tb` — JSONL を TensorBoard へ

```bash
uv run jsonl-to-tb --input runs/selfplay_20260510_120000/all.jsonl \
                   --output runs/tb/

tensorboard --logdir runs/tb/
```

エクスポートされる scalar:

- `train/black_win_rate` ( 移動平均 )
- `train/episode_length`
- `train/score_diff` ( Black − White )

`tensorboard --logdir runs/tb/` で閲覧できます．複数の入力ファイルは TB UI 上では別々の run として表示されるので，バッチ間で構成を比較できます．

## まとめて使う

典型的なワークフロー:

```bash
# 1. データ生成．
othello-cli selfplay \
    --num-games 5000 --threads 8 \
    --black mcts:200 --white random:seed=1 \
    --seed 42 --log-dir auto --swap-colors

# 2. 簡易統計．
uv run analyze-stats \
    --input runs/selfplay_20260510_120000/ \
    --output stats.csv

# 3. 過去のバッチと比較．
uv run analyze-compare \
    --a runs/selfplay_20260509_180000/ \
    --b runs/selfplay_20260510_120000/

# 4. 学習 scalar を TensorBoard に流し込む．
uv run jsonl-to-tb \
    --input runs/selfplay_20260510_120000/all.jsonl \
    --output runs/tb/

# 5. 代表局を GIF にして論文や発表資料に使う．
uv run plot-game \
    --input runs/selfplay_20260510_120000/game_0042.json \
    --gif game_0042.gif
```

## 関連項目

- [Self-play & バッチ実行](self-play.md) — これらのツールが消費する入力を生成する．
- [レコードフォーマット](record-formats.md) — JSON / GGF / JSONL のスキーマ．
- 各パッケージ ( `tools/visualize/` ， `tools/analyze/` ， `tools/tb_converter/` ) の README — パッケージレベルのインストールと API リファレンス．
