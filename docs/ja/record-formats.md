[English](../record-formats.md) | [日本語](record-formats.md)

# レコードフォーマット

`rs-othello-sim` は，それぞれ異なる用途に最適化された複数の棋譜フォーマットを読み書きします．インメモリのニュートラル表現は `othello_io::GameRecord` で，すべての reader はこの型を生成し，すべての writer はこの型を消費します．

| フォーマット | 読み込み | 書き込み | 用途 |
|---|---|---|---|
| 自己記述 JSON | yes | yes | デフォルト．全メタデータ＋手ごとのタイムスタンプを保持 |
| GGF ( Othello サブセット ) | yes | yes | 既存 Othello ツールとの相互運用 |
| WTHOR ( `.wtb` ) | yes | no | 公開 Othello データベースの取り込み |
| JSONL イベントログ | append | append | ストリーミングログ ( 1 行 1 イベント ) |

変換サブコマンド `othello-cli convert` は任意の reader フォーマットを受け取り，任意の writer フォーマットを出力できます． [`docs/cli-usage.md#convert--棋譜フォーマットの変換`](cli-usage.md#convert--棋譜フォーマットの変換) を参照してください．

## 自己記述 JSON

デフォルトのフォーマットは `schema_version` ， `metadata` ， `moves` 配列を持つ単一の JSON オブジェクトです．現状の schema version は `"1.0"` です．

```json
{
  "schema_version": "1.0",
  "metadata": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "started_at": "2026-05-09T15:30:00.000+09:00",
    "ended_at":   "2026-05-09T15:30:42.123+09:00",
    "board_size": { "rows": 8, "cols": 8 },
    "players": {
      "black": { "name": "MctsPlayer",   "params": { "simulations": 1000 } },
      "white": { "name": "RandomPlayer", "params": { "seed": 42 } }
    },
    "result": { "winner": "Black", "score": { "black": 38, "white": 26 } },
    "engine_version": "rs-othello-sim 0.1.0"
  },
  "moves": [
    { "n": 1, "side": "Black", "move": { "Place": { "row": 2, "col": 3 } },
      "ts": "2026-05-09T15:30:00.123+09:00" },
    { "n": 2, "side": "White", "move": { "Place": { "row": 2, "col": 2 } },
      "ts": "2026-05-09T15:30:00.456+09:00" }
  ]
}
```

注意点:

- `move` は通常の着手なら `{ "Place": { "row": R, "col": C } }` ，パスなら `"Pass"` ( 文字列 ) ．
- タイムスタンプは RFC 3339 ( ミリ秒精度，タイムゾーンオフセット付き ) ．
- reader は major schema version が自身と異なるレコードを拒否します．

## GGF ( Othello サブセット )

GGF ( Generic Game Format ) はコミュニティで長く使われている形式です．本リポジトリでは Othello サブセットを実装しています:

```
(;GM[Othello]PC[rs-othello-sim]DT[2026-05-09]
 PB[MctsPlayer]PW[RandomPlayer]
 RE[+12]
 BO[8 ---------------------------O*------*O--------------------------- *]
 B[D3//1.234]W[C5//0.456]B[E3//1.111]…;)
```

認識するタグ:

| タグ | 意味 |
|---|---|
| `GM` | ゲーム名 ( `Othello` のみ ) |
| `PB` / `PW` | Black / White プレイヤー名 |
| `PC` | Place ( 棋譜を生成した Engine ) |
| `DT` | 日付 ( 自由記述 ) |
| `RE` | 結果 ( `+N` / `-N` ，引き分けは `=` ) |
| `BO` | 初期盤面 ( `size board-string side-to-move` ) |
| `B[…]` / `W[…]` | 着手 ( 座標．後ろに `//time` を付けることもある ) |

コメント ( `C[…]` ) や variations は出力しません．reader は知らないタグを黙って読み飛ばします．

## WTHOR ( `.wtb` ) — 読み取り専用

WTHOR は主要 Othello データベース ( 例: Frédéric Donninger コレクション ) で使われる固定長バイナリアーカイブです．レイアウトは:

- 16 byte ヘッダ ( 年，対局数など ) ．
- 1 局あたり 68 byte: メタデータ 8 byte ＋ 着手 60 byte ．
- 各着手は 1 byte で `(row - 1) * 10 + col` ( 1-indexed ) ． `0` は null move ( 60 byte バッファの未使用末尾を示す ) ．

reader は読み取り専用です．編集が必要な場合は先に JSON または GGF へ変換してください． `inspect` サブコマンドは `.wtb` を展開せずにアーカイブ全体の集計を表示できます．

## JSONL イベントログ

`othello_io::JsonlLogger` は 1 行 1 JSON オブジェクトで出力します．ストリーミングや `tail` での閲覧に最適化されています．バッチ self-play は局ごとの棋譜と並行してこれを 1 本生成します．

```jsonl
{"event":"game_start","ts":"2026-05-09T15:30:00.000+09:00","game_id":"550e…","board_size":[8,8],"players":{"black":"Mcts","white":"Random"}}
{"event":"move","ts":"…","game_id":"550e…","n":1,"side":"Black","move":{"Place":[2,3]},"stones":{"black":4,"white":1},"legal_count":3}
{"event":"move","ts":"…","game_id":"550e…","n":2,"side":"White","move":{"Place":[2,2]},"stones":{"black":3,"white":3},"legal_count":4}
{"event":"pass","ts":"…","game_id":"550e…","n":30,"side":"Black"}
{"event":"game_end","ts":"…","game_id":"550e…","winner":"Black","stones":{"black":38,"white":26},"moves_total":60}
```

`move` イベントは JSON 棋譜中の `{Place: {row, col}}` ではなくコンパクトな `[row, col]` 配列を使います．これは意図的な選択です．JSONL はストリームと集約のための形式なので行長を短く保ちたく，棋譜 JSON はアーカイブと往復変換のための形式なのでスキーマを自己記述的にしています．

イベントの種類は `game_start` ， `move` ， `pass` ， `game_end` の 4 種です．各行は `event` ， `ts` ， `game_id` を必ず含み，残りは種類ごとに異なります．

## 往復変換の例

WTHOR アーカイブ → 個別 JSON ファイル:

```bash
othello-cli convert \
  --input archive.wtb --input-format wthor \
  --output-format json --output-dir converted/
```

GGF → JSON:

```bash
othello-cli convert \
  --input game.ggf --input-format ggf \
  --output-format json --output game.json
```

JSON → GGF ( 別の Othello ツールに渡す場合など ):

```bash
othello-cli convert \
  --input game.json --input-format json \
  --output-format ggf --output game.ggf
```

JSONL は棋譜アーカイブではなくイベントログのため， `convert` の出力には含まれません．JSONL を生成したい場合は `selfplay --jsonl-log PATH` を使ってください．

## 関連項目

- [Self-play & バッチ実行](self-play.md) — `--log-dir auto` が生成するディレクトリレイアウト．
- [ツール ( visualize / analyze / TB )](tools-visualize.md) — 両フォーマットを消費する Python ユーティリティ群．
- `crates/othello-io/src/` — reader / writer のソース．
