[English](../contributing.md) | [日本語](contributing.md)

# コントリビュート

このリポジトリで採用している規約の簡潔なガイドです．フェーズごとの実際の履歴は [CHANGELOG.md](../../CHANGELOG.md) を，より上位の意図は設計書 ( 親 Obsidian vault の `設計書/Othello_シミュレータ設計書.md` ) を参照してください．

## 開発環境

- **Rust** 1.85 以上 ( `Cargo.toml` の `rust-version` は 1.85 ， edition 2024 ) ．
- **Python** 3.11 以上 — `tools/` ワークスペースと `othello-py` 用．
- **`uv`** Python パッケージ管理用． PyO3 拡張には `maturin` ．Rust ワークスペース単体は Python に依存しません．

```bash
# 初回セットアップ
cargo build --workspace
uv sync --all-packages

# オプション: PyO3 バインディング
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## ブランチとコミット

現状の運用: `main` に直接コミット・プッシュ．長期のフィーチャーブランチは維持していません．フェーズはコミットメッセージと `CHANGELOG.md` の記述で追跡します．各コミットは 1 つの首尾一貫した変更にとどめ，関連するならフェーズ名を本文で参照してください．

## テスト

push 前にローカルで Rust テスト一式を実行してください:

```bash
cargo test --workspace                       # Phase 6.7 時点で 331 + 5 ignored
cargo test --workspace -- --include-ignored  # 外部 Engine をセットアップしているときのみ
```

テストレイヤ ( [設計 §8.1–8.3](#) ):

| レイヤ | ツール | 場所 |
|---|---|---|
| ユニット | `cargo test` | 各クレートの `src/` モジュール |
| 統合 | `cargo test` | `crates/*/tests/` |
| プロパティ | `proptest` | `crates/othello-core/tests/property_tests.rs` |
| スナップショット | `insta` | `crates/othello-tui/tests/snapshot.rs` |
| スモーク ( ゲート付き ) | `#[ignore]` | `crates/othello-player/tests/real_engine.rs` ( Edax / Egaroucid ) |

Python テスト:

```bash
uv run pytest                              # tools/* と othello-py 全体
uv run pytest tools/visualize/tests/       # 個別パッケージ
```

新フィーチャ追加時は，それが属するレイヤにテストを追加してください．新しい public 型はユニットテスト，新しいコマンドは統合テスト，新しいファイルフォーマットはラウンドトリッププロパティテストを伴うべきです．

## Lint ， format ， doc

CI は厳格バリアントを実行します．コミット前にローカルでも同じものを実行してください:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo doc --workspace --no-deps           # warning は failure 扱い
```

Python:

```bash
uv run ruff check tools/ crates/othello-py/
uv run ruff format --check tools/ crates/othello-py/
```

ワークスペース全体で `unsafe` は使用していません．導入する場合は明示的な justification が必要です．

## 新フィーチャ追加 — チェックリスト

1. **設計**． 外部から見える契約に変更が及ぶ場合は `設計書/Othello_シミュレータ設計書.md` を更新．
2. **実装**． クレート境界を清潔に保つ ( [architecture.md](architecture.md) を参照 ) ．重い新規依存 ( Candle ， Tokio など ) は，他クレートがコンパイルコストを払わずに済むよう独立クレートに置く．
3. **テスト**． ユニット＋統合．新たな不変条件があればプロパティ．TUI を触ったらスナップショット．
4. **ドキュメント**． `docs/` 配下のファイルを更新または新規作成し，関連ガイドからクロスリンクを張る ( [README](../../README.md) はトップレベル docs にしかリンクしない ) ．
5. **CHANGELOG**． 現在のフェーズ下に `### Added` / `### Changed` / `### Fixed` のエントリを追加．
6. **検証**:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test --workspace
   cargo doc --workspace --no-deps
   ```

## フェーズ規約

フェーズはおおむね設計書のマイルストーンに対応します．各フェーズで生まれるもの:

- 一貫した CHANGELOG エントリ群．
- 新しい表面に対するテストカバレッジ．
- ドキュメント更新 ( しばしば `docs/` 配下に新ファイル ) ．

Phase 6 のサブタスク ( 6.1 ， 6.2 ， … ) は独立しており，現時点で価値の高いものから着手して構いません．設計書 §10 に一覧があります．

## ドキュメント責務

- public な CLI フラグに触れる → [`docs/cli-usage.md`](cli-usage.md) を更新．
- 新しい Player を追加 → [`docs/cli-usage.md`](cli-usage.md) と該当の詳細ドキュメント ( `docs/external-engines.md` ， `docs/nn-evaluator.md` 等 ) を更新．
- 新しいファイルフォーマットを追加 → [`docs/record-formats.md`](record-formats.md) を更新．
- クレート依存を変更 → [`docs/architecture.md`](architecture.md) の mermaid を更新．
- ユーザに見える変更全般 → 独立ページに値するものは [README](../../README.md) の docs インデックスに必ず載せる．

ルート README は意図的に短く保たれています ( イントロ＋インストール＋クイックスタート＋ docs インデックス ) ．肥大化させず，詳細は `docs/` に置いてください．
