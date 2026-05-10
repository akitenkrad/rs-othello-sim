[English](../tui-guide.md) | [日本語](tui-guide.md)

# TUI ガイド

`othello-tui` は [ratatui](https://ratatui.rs/) ベースのフロントエンドで，`othello-cli` の `play` / `replay` / `observe` サブコマンドから起動されます．3 つの異なるモードを持ちます:

| モード | サブコマンド | 用途 |
|---|---|---|
| Play | `othello-cli play` ( TUI 版 ) | 同一ターミナルで人間 2 名が交互に着手 |
| Replay | `othello-cli replay --file …` | 保存済みの棋譜を 1 手ずつ再生 |
| Observe | `othello-cli observe …` | AI 同士の対局を Evaluator overlay 付きで観戦 |

Observe モードの特徴は，現在手番の Player が `Evaluator` を実装している場合に，リアルタイムの Evaluator 情報を表示できる点です ( 現状 MCTS と `nn:` 系が対応 ) ．

## レイアウト

3 モード共通の骨格は次のとおりです:

```
+------------------+----------------------+
|                  | Players              |
|                  | Black: …             |
|     Board        | White: …             |
|     ( 中央 )       +----------------------+
|                  | Status / Help        |
|                  | ( モード固有の行 )      |
|                  +----------------------+
|                  | Evaluator (Observe)  |
|                  | move  visits  bar    |
|                  +----------------------+
+------------------+----------------------+
| フッターキーマップ                          |
+------------------------------------------+
```

Board パネルは 1 セル 1.5 × 1 ( 横 2 列 ) で描画され，石が丸く見えるようになっています．Black は `B` ， White は `W` ，現在手番の合法手はドットでハイライトされます．

## Play モード

`othello-cli play` を TUI フロントエンド付きでビルドした際に使用します．両プレイヤーが同じターミナルで交互に着手します．

| キー | アクション |
|---|---|
| 矢印キー / `h j k l` | カーソル移動 |
| `Enter` / `Space` | カーソル位置に着手 |
| `p` | パス ( 合法手がないときのみ有効 ) |
| `q` / `Esc` | 終了 |

ステータスバーには現在手番，合法手数，カーソルが非合法マスにあるときの警告が表示されます．

## Replay モード

```bash
othello-cli replay --file game.json --format json

# 250 ms/手のテンポで自動再生から開始
othello-cli replay --file game.json --format json --auto --auto-delay 250
```

棋譜全体を `Replayer` に読み込みます．各 ply のフルスナップショットを保持しているためナビゲーションは `O(1)` です．

| キー | アクション |
|---|---|
| `→` / `l` | 1 手進む |
| `←` / `h` | 1 手戻る |
| `0` | 開始位置へ |
| `$` | 終局位置へ ( 自動再生も停止) |
| `Space` / `a` | 自動再生の ON/OFF．終局位置で押すと先頭から再開 |
| `+` / `=` | 自動再生間隔を 100 ms 増やす ( 上限 5000 ms) |
| `-` / `_` | 自動再生間隔を 100 ms 減らす ( 下限 50 ms) |
| `q` / `Esc` | 終了 |

ヘッダには `Move N / TOTAL` とスコアが表示され，自動再生中は `[AUTO <delay>ms]` が併記されます．自動再生は最終手に到達すると自動的に停止するため，終局局面をしばらく確認してから終了できます．メタデータ ( プレイヤー名，タイムスタンプ ) を含む棋譜は，右側の Players パネルに表示されます．

## Observe モード

```bash
othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

両 AI を起動し対局を TUI で表示します． `--auto-delay 0` ( デフォルト ) では各 ply で `Space` 押下を待ちます． `--auto-delay 500` では 500 ms 後に次手を要求します．

| キー | アクション |
|---|---|
| `Space` | 1 手進める ( 手動モード ) |
| `q` / `Esc` | 終了 |

**Evaluator overlay** は，現在手番の Player が `Player::evaluator()` を公開しているときに Players パネルの下に表示されます． `MctsPlayer` の場合，直近の `select_move` 終了時点で記録された正規化済みルート訪問数を，`NnEvaluator` の場合はマスク後・正規化済みのポリシーを表示します:

```
Evaluator (top 5)
  D5    0.42  ███████░░░░░
  E6    0.21  ████░░░░░░░░
  F4    0.19  ███░░░░░░░░░
  C5    0.10  ██░░░░░░░░░░
  pass  0.08  █░░░░░░░░░░░
```

バーは最大値が幅いっぱいになるよう正規化されます． overlay は `select_move` から戻ってきた後にだけ更新され，探索中は何も表示されません．

## スナップショットテスト

TUI のレンダリングは [insta](https://insta.rs/) のスナップショットテスト ( `crates/othello-tui/tests/snapshot.rs` ) で固定されています．レイアウトを変える場合は `cargo insta review` でスナップショットを再生成してください．

## デモ GIF

ルート README に埋め込むメインのデモは `docs/assets/demo.gif` です．モード別キャプチャを追加する場合は同ディレクトリに以下の名前で配置:

- `tui-play.gif` — Play モード
- `tui-observe.gif` — Observe モード

撮影手順は [`docs/assets/README.md`](../assets/README.md) ( macOS の画面収録，または asciinema + agg ) を参照してください．
