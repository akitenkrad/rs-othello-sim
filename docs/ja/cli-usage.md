[English](../cli-usage.md) | [日本語](cli-usage.md)

# CLI 利用ガイド

`othello-cli` は TUI 以外の全ワークフローを駆動する単一バイナリです．人間同士の対局，シミュレーション対局，バッチ self-play，棋譜の変換と検査，マイクロベンチマーク，AI 同士の対戦観戦用 Observe TUI までをカバーします．

このページはレシピ形式のツアーです．より個別のガイドは以下を参照してください:

- [Self-play & バッチ実行](self-play.md) — `selfplay` サブコマンドの詳細．
- [TUI ガイド](tui-guide.md) — Play / Replay / Observe の各モード．
- [レコードフォーマット](record-formats.md) — `simulate` / `convert` / `inspect` が扱うファイル形式．
- [外部 Engine](external-engines.md) — `external:` PlayerSpec の使い方．
- [NN Evaluator](nn-evaluator.md) — `nn:` PlayerSpec の使い方．

## グローバルフラグ

これらのオプションはすべてのサブコマンドに共通です．

| フラグ | デフォルト | 説明 |
|---|---|---|
| `--log-level <LEVEL>` | `info` | `trace` / `debug` / `info` / `warn` / `error` |
| `--log-format <FORMAT>` | `text` | `text` ( 人間向け ) または `json` ( 構造化 ) |
| `--log-file <PATH>` | _なし_ | stderr に加えて `PATH` に JSON 形式の tracing イベントを書き出す |

ログは stderr に出力されます．ファイルに残したい場合はシェルの `2>` でリダイレクトしてください． `--log-file` は `--log-format` の指定にかかわらず JSON で書き込みます．

## `play` — stdin/stdout で人間 vs 人間

```bash
othello-cli play --board-size 8
```

両者を交互に促します． `d3` や `c4` のように座標を入力します．ビルドの動作確認に手早く使えます．インタラクティブ UI が欲しい場合は [`replay`](#replay--tui-での棋譜再生) や [`observe`](#observe--tui-で-ai-vs-ai) を使ってください．

## `simulate` — 1 局のシミュレーション

```bash
othello-cli simulate \
  --board-size 8 \
  --black random:seed=42 \
  --white greedy \
  --save-record game.json \
  --record-format json
```

| フラグ | 説明 |
|---|---|
| `--board-size N` | 4..=26 ，デフォルト 8 |
| `--black SPEC` / `--white SPEC` | Player spec ( [文法](#playerspec-文法) を参照 ) |
| `--save-record PATH` | 棋譜の出力先 ( 任意 ) |
| `--record-format json\|ggf` | 保存時のフォーマット |
| `--jsonl-log PATH` | JSONL イベント ( `game_start` / `move` / `pass` / `game_end` ) を追記 |

対局はインプロセスで実行されます．バッチ実行には [`selfplay`](self-play.md) を使ってください．

## `replay` — TUI での棋譜再生

```bash
othello-cli replay --file game.json --format json
othello-cli replay --file game.ggf  --format ggf

# 250 ms/手のテンポで自動再生から開始
othello-cli replay --file game.json --format json --auto --auto-delay 250
```

棋譜を TUI の Replay 画面に読み込みます．各 ply のスナップショットを保持しているため，1 手前進・後退・任意の手数へのジャンプはすべて `O(1)` です．WTHOR は [`convert`](#convert--棋譜フォーマットの変換) で変換してから読み込んでください．

| フラグ | 効果 |
|---|---|
| `--auto` | 起動直後から自動再生を開始 ( 既定では `Space` / `a` 待ち) |
| `--auto-delay MS` | 自動再生時の手間隔 ( ミリ秒，既定 `500`，範囲 `[50, 5000]`) ．モード内で `+` / `-` により調整可能 |

Replay 画面内では `Space` または `a` で自動再生 ON/OFF， `+` / `-` で間隔を 100 ms 単位で増減， `0` / `$` で先頭 / 末尾へジャンプ， `<-` / `->` ( または `h` / `l`) で手動進退します．キー一覧は [TUI ガイド](tui-guide.md#replay-モード) を参照してください．

## `fetch` — 公開データセットのダウンロード

```bash
# 対応データセットの一覧
othello-cli fetch list

# 単一年
othello-cli fetch wthor --year 2023 --dest data/wthor/

# 範囲指定 ( 両端含む)
othello-cli fetch wthor --years 2020..2023 --dest data/wthor/

# 既に展開済みの年を強制再ダウンロード
othello-cli fetch wthor --year 2023 --force

# URL パターンをカスタムに ( FFO のレイアウトが変わった場合)
othello-cli fetch wthor --year 2023 \
  --url-pattern 'https://example.org/wthor/wth_{YEAR}.zip'
```

`wthor` プロバイダは [フランスオセロ連盟 ( FFO)](https://www.ffothello.org/informatique/la-base-wthor/) から年次アーカイブをダウンロードし， `.wtb` ( および付随する `.JOU` / `.TOU`) を `--dest` 直下に展開します．展開後は途中の `.zip` を削除します．

| フラグ | デフォルト | 説明 |
|---|---|---|
| `--year N` | _( 必須)_ | 単一年 ( 例 `--year 2023`)．`--years` と排他 |
| `--years A..B` | _( 必須)_ | 両端含む範囲 ( 例 `--years 2020..2023`)．`--year` と排他 |
| `--dest PATH` | `data/wthor/` | 展開先ディレクトリ ( 未作成なら作成) |
| `--force` | `false` | 既に `wth_YYYY.wtb` が展開済みでも再取得 |
| `--keep-archive` | `false` | ダウンロードした `.zip` を削除しない |
| `--url-pattern URL` | _( なし)_ | URL テンプレートを上書き．`{YEAR}` プレースホルダ必須 |
| `--no-progress` | _( 自動)_ | 進捗バーを抑制 ( 既定: stderr が tty なら表示) |

### URL パターンの解決順

最初に見つかった非空の値が使われます:

1. `--url-pattern` フラグ
2. 環境変数 `OTHELLO_WTHOR_URL_PATTERN`
3. 組み込みデフォルト `https://www.ffothello.org/wthor/wth_{YEAR}.zip`

FFO 側でアーカイブの配置が変わって既定 URL が 404 を返す場合，CLI は失敗 URL とともに <https://www.ffothello.org/informatique/la-base-wthor/> と `--url-pattern` / 環境変数による上書き方法を提示します．新しい URL を確認するか，`curl` + `unzip` で手動取得してください．

### ライセンスについて

WTHOR は **Fédération Française d'Othello** が研究・非商用利用向けに配布しています．再配布や論文掲載時は FFO とアーカイブの年を必ずクレジットしてください．詳細は [`docs/external-data.md`](external-data.md) のライセンス・引用チェックリストを参照．

## `convert` — 棋譜フォーマットの変換

```bash
# 単一棋譜
othello-cli convert \
  --input game.wtb --input-format wthor \
  --output-format json --output game.json

# 一括変換 ( 1 ファイル 1 局 )
othello-cli convert \
  --input archive.wtb --input-format wthor \
  --output-format ggf --output-dir converted/

# JSON から GGF へ
othello-cli convert \
  --input game.json --input-format json \
  --output-format ggf --output game.ggf
```

| フラグ | 説明 |
|---|---|
| `--input PATH` | 入力ファイル |
| `--input-format json\|ggf\|wthor` | 入力フォーマット |
| `--output-format json\|ggf` | 出力フォーマット ( WTHOR は読み取り専用 ) |
| `--output PATH` | 単一レコード出力 ( `--output-dir` と排他 ) |
| `--output-dir DIR` | 一括出力先ディレクトリ．1 局につき 1 ファイル |

WTHOR アーカイブには複数局が含まれることが多いため， `--output-dir` で各局を別ファイルに展開します．フォーマットの詳細は [record-formats.md](record-formats.md) にあります．

## `inspect` — 棋譜の統計

```bash
# 単一の JSON / GGF
othello-cli inspect --file game.json --format json

# WTHOR アーカイブ全体の集計
othello-cli inspect --file archive.wtb --format wthor
```

盤面サイズ，手数，勝者，スコア，および ( WTHOR の場合は ) 総局数と結果別の集計を表示します．変換済みアーカイブを [analyze ツール](tools-visualize.md) に流す前のサニティチェックに便利です．

## `selfplay` — バッチ self-play

詳細なウォークスルーは [self-play.md](self-play.md) にあります．最小例:

```bash
othello-cli selfplay \
  --num-games 1000 --threads 8 \
  --black mcts:200 --white random:seed=1 \
  --seed 42 --log-dir auto --swap-colors
```

ハイライト:

- `--log-dir auto` は `runs/selfplay_YYYYMMDD_HHMMSS/` に局ごとの JSON 棋譜と `all.jsonl` を出力します．
- `--threads 0` ( デフォルト ) は Rayon を介して論理コア数すべてを使います．
- stderr が TTY のとき `indicatif` のライブ進捗バーが表示されます． `--no-progress` で無効化できます．
- `--swap-colors` は奇数局と偶数局で Black / White を入れ替えます．

## `benchmark` — マイクロベンチマーク

```bash
othello-cli benchmark --target legal-moves --duration 5
othello-cli benchmark --target self-play --board-size 8 --duration 5
othello-cli benchmark --target self-play --board-size 16 --duration 5
```

`--duration` 秒で平均された ops/sec または moves/sec を表示します．statistics 付きの criterion ベンチマークについては [benchmarks.md](benchmarks.md) を参照してください．

## `observe` — TUI で AI vs AI

```bash
othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

Observe モード ( [TUI ガイド](tui-guide.md) を参照 ) を起動します．現在手番側の Player が `Evaluator` を実装している場合は Evaluator overlay が表示されます ( MCTS と `nn:` 系は実装済，Random と Greedy は非対応 ) ．

| フラグ | 説明 |
|---|---|
| `--auto-delay MS` | `0` は次手まで `Space` を待つ． `>0` は自動進行 |
| `--seed N` | 確率的 Player のための seed ( 任意 ) |

## PlayerSpec 文法

`--black` と `--white` は以下の文法を受け付けます:

```
random[:seed=N]
greedy
mcts:N[,c=F][,seed=M][,depth=D][,tree_reuse=true]
external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,...][,command=PATH]
nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
```

| Spec | 補足 |
|---|---|
| `random` | 合法手の中から一様ランダムに選択 |
| `random:seed=N` | RNG seed を固定 |
| `greedy` | 即座にひっくり返せる石数を最大化．同点は座標で決定 |
| `mcts:N` | UCT MCTS．1 手あたり `N` simulations |
| `mcts:N,c=F` | UCT exploration 定数 ( デフォルトは `sqrt(2)` ) |
| `mcts:N,seed=M` | rollout を決定論的にする |
| `mcts:N,depth=D` | rollout の深さを制限 |
| `mcts:N,tree_reuse=true` | 手をまたいで部分木を再利用 ( Phase 6.2 ) |
| `external:PATH` | サブプロセス Engine ． [external-engines.md](external-engines.md) を参照 |
| `nn:safetensors:PATH` | Candle 経由でロードする NN evaluator ( Phase 6.4 ) |
| `nn:onnx:PATH` | ONNX 経由でロードする NN evaluator |

`nn:` の `temperature=F` と `deterministic` は [nn-evaluator.md](nn-evaluator.md) で説明しています． `human` Player に spec はありません．`play` モードが自動的に組み込みます．
