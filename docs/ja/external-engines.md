[English](../external-engines.md) | [日本語](external-engines.md)

# 外部 Engine

`rs-othello-sim` は，サードパーティの Othello Engine を `ExternalEnginePlayer` 経由で Player として利用できます．本ドキュメントの内容は以下のとおりです:

1. なぜ外部 Engine をサポートするか．
2. サポートしている 2 種類のワイヤープロトコル ( `gtp` と `ntest` ) ．
3. `external:` `PlayerSpec` 文法による Engine の組み込み方．
4. `scripts/fetch_engines.sh` を使ってローカルに実 Engine ( Edax / Egaroucid ) を取得・ビルドする方法．
5. バイナリを同梱しないライセンス上の理由．
6. ゲート付き ( gated ) の実 Engine スモークテストの実行方法．

## なぜ外部 Engine が必要か

組み込み Player ( `random` / `greedy` / `mcts` ) はほとんどのシミュレーション要件をカバーしますが，研究用途では強力でよく知られたリファレンス対戦相手が必要になることがあります:

- **キャリブレーション**: 内製 MCTS Player が一定レベルの Edax 4.4 とどの程度戦えるかを測定する．
- **Self-play データの品質向上**: NN evaluator ( Phase 6.4 ) を Edax / Egaroucid のスコアラベル付き局面でブートストラップする．
- **実装間のリグレッションチェック**: 内製ルールエンジンが既存プログラムと一致するか確認する．

`ExternalEnginePlayer` は Engine をサブプロセスとして起動し，stdin / stdout で同期通信します．1 手あたりのタイムアウトは worker thread と `mpsc::channel` で実装しており，`tokio` 依存はありません．

## モック Engine と実 Engine

テストは 2 層構成です:

| レイヤ | 場所 | CI で実行? | 目的 |
|---|---|---|---|
| モック Engine ( bash ) | `crates/othello-player/tests/mock_engine_*.sh` + `external_engine.rs` | Yes | プロトコルレベルの適合性確認．高速 |
| 実 Engine ( Edax / Egaroucid ) | `crates/othello-player/tests/real_engine.rs` | No ( `#[ignore]` ) | 実装に対するスモークテスト |

モックスクリプトは各プロトコルの最低限の表面のみを実装しており，ネイティブバイナリ無しでもリクエスト / レスポンスのフレーミングを統合テストで検証できます．実バイナリはユーザがオンデマンドで取得します．

## `external:` PlayerSpec 文法

```
external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,arg=VAL,...][,command=PATH]
```

| キー | 必須 | デフォルト | 説明 |
|---|---|---|---|
| ( 位置引数 ) | 位置引数か `command=` のいずれか | - | Engine 実行ファイルへのパス |
| `command` | 位置引数か `command=` のいずれか | - | 位置引数と同じ．key=value 形式 |
| `protocol` | No | `gtp` | `gtp` または `ntest` ( エイリアス: `edax` ， `egaroucid` → `ntest` ) |
| `timeout` | No | `30` | 1 手あたりのタイムアウト ( 秒 ) |
| `arg` | No ( 繰り返し可 ) | - | Engine に転送する追加 CLI 引数 |

例:

```text
external:/usr/local/bin/edax
external:./engines/edax,protocol=ntest,timeout=10
external:./engines/egaroucid,protocol=gtp,arg=--level,arg=1
external:command=/opt/edax/bin/lEdax-x64-modern,protocol=ntest
```

CLI では他の Player spec と同様に使えます:

```bash
cargo run -p othello-cli -- simulate \
  --black "external:./vendor/engines/edax/bin/lEdax-x64-modern,protocol=ntest" \
  --white "mcts:200,seed=1" \
  --board-size 8
```

## プロトコル

### GTP ( Go Text Protocol スタイル，デフォルト )

おおまかには次のようなやり取りです:

```
> boardsize 8
< =
> clear_board
< =
> play black D3
< =
> genmove white
< = D5
> quit
< =
```

各リクエストは 1 行で送り，レスポンスは `=` ( 成功 ) または `?` ( エラー ) で始まります． `pass` は座標トークンとして有効です．

### Ntest ( Edax / Egaroucid 簡易版 )

おおまかには次のようなやり取りです:

```
> set game <board_str>
< OK
> go
< D5
> quit
```

Edax の `--ggs` モードや Egaroucid の console モードはどちらもこのバリエーションを受け付けます．アダプタは `crates/othello-player/src/external/ntest.rs` にあります．

## 実 Engine の取得

`scripts/fetch_engines.sh` は Engine のバイナリを `vendor/engines/` 以下にダウンロード ( Edax はビルド ) します．このディレクトリは `.gitignore` 済みです．

```bash
# Edax 4.4 ( git clone + make でビルド )
bash scripts/fetch_engines.sh edax

# Egaroucid ( ビルド済みリリース tarball ．URL はスクリプト内で要編集 )
bash scripts/fetch_engines.sh egaroucid

# 両方
bash scripts/fetch_engines.sh all

# 強制再インストール ( 既存の vendor/engines/<name>/ を削除 )
bash scripts/fetch_engines.sh all --force
```

スクリプトは以下を自動判別します:

- `Darwin-arm64` → Edax `ARCH=arm BUILD=osx`
- `Darwin-x86_64` → Edax `ARCH=x64-modern BUILD=osx`
- `Linux-x86_64` → Edax `ARCH=x64-modern BUILD=linux`
- `Linux-aarch64` → Edax `ARCH=arm BUILD=linux`

各 Engine ディレクトリには取得元とライセンスを記した `LICENSE-NOTE.md` が置かれます．

### Egaroucid のリリース URL に関する注意

Egaroucid のリリースアーカイブ名はバージョンごとに変わるため，スクリプトでは各プラットフォーム用の URL に `<TODO: fill release URL>` プレースホルダが入っています． `bash scripts/fetch_engines.sh egaroucid` の前に `scripts/fetch_engines.sh` を編集して，対象プラットフォーム用の URL を <https://github.com/Nyanyan/Egaroucid/releases> から取得し置き換えてください．

### Edax の `eval.dat`

Edax は実行時に評価ファイル ( `eval.dat` ) を必要とします．このファイルはソースとは別に <https://www.abulmo.perso.neuf.fr/edax/4.4/> や <https://eukaryote31.github.io/edax/> から配布されており，GitHub のソースリポジトリには同梱されていません．スクリプト実行後に `eval.dat` を別途ダウンロードし，バイナリからの相対パスで Edax が参照できるよう `vendor/engines/edax/data/eval.dat` に置いてください．

## ライセンス上の注意

本リポジトリでは Edax / Egaroucid のバイナリを意図的に再配布していません:

- **Edax**: GPL-2.0 ．ソースは公開されていますが `eval.dat` は独自配布で，ユーザが別途取得する必要があります．
- **Egaroucid**: GPL-3.0 ．作者公開のリリースバイナリはダウンロードできますが，第三者プロジェクトが再ホストすべきではありません．

`scripts/fetch_engines.sh` はユーザのマシン上でローカルに動作する *fetch ヘルパー* です．生成物は `.gitignore` 済みの `vendor/engines/` に置かれるため，Engine 固有のファイルがコミットに含まれることはありません．

## ゲート付きスモークテストの実行

実 Engine の統合テストは `crates/othello-player/tests/real_engine.rs` にあります．すべて `#[ignore = "..."]` 付きなので，普通の `cargo test` ではスキップされます．

```bash
# 1. Engine の取得 ( 1 回のみ ):
bash scripts/fetch_engines.sh all

# 2. スモークテストの実行:
cargo test -p othello-player --test real_engine -- --ignored
```

2 つの Engine のうち片方だけがインストールされている場合，対応するテストは `[skip] ... not installed` と出力して何もせずに pass します．部分的なインストールでも問題ありません．

各テストの確認内容:

| テスト | 必要なもの | 確認内容 |
|---|---|---|
| `edax_returns_a_legal_move` | `vendor/engines/edax/bin/...` | Edax が初期局面に対し合法手を返す |
| `egaroucid_returns_a_legal_move` | `vendor/engines/egaroucid/...` | Egaroucid が初期局面に対し合法手を返す |
| `edax_vs_egaroucid_short_match` | 両方 | Edax ( Black, ntest ) vs Egaroucid ( White, gtp ) で 6 ply のプロトコルエラー・非合法手なし |

クロス Engine 対局は 6 ply のみで結果はアサートしません．プロトコル相互運用性のスモークテストであり，強さのベンチマークではありません．
