# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## リポジトリ概要

`rs-othello-sim` は Othello (Reversi) のシミュレーション研究および強化学習実験のための Rust ワークスペースである．設計書は親 Obsidian vault の `設計書/Othello_シミュレータ設計書.md` を参照すること．

現在は **Phase 1〜5 すべて完了**．`crates/` 配下の 8 つの Rust クレートと，`tools/` 配下の 3 つの Python ツール ( uv workspace) で構成されている．

## ワークスペース構成

```
rs-othello-sim/
├── Cargo.toml                  # Cargo workspace manifest
├── pyproject.toml              # uv workspace root ( tools/)
├── CHANGELOG.md                # Phase 1〜5 の変更履歴
├── crates/
│   ├── othello-core/           # 盤面・ルール・状態 ( Bitboard8 / GenericBoard / Hybrid)
│   ├── othello-player/         # Player + Evaluator trait + Random/Greedy/Human/MCTS/External
│   │   └── src/external/       # ExternalEnginePlayer + GtpProtocol + NtestProtocol
│   ├── othello-io/             # GameRecord + JSON / GGF / WTHOR / JSONL ロガー
│   ├── othello-engine/         # GameEngine, GameHistory, Replayer, BatchRunner, ProgressCallback
│   ├── othello-rl/             # Gymnasium / PettingZoo 互換 environment
│   ├── othello-tui/            # ratatui Play / Replay / Observe ( Evaluator overlay)
│   ├── othello-cli/            # CLI バイナリ
│   └── othello-py/             # PyO3 Python バインディング
├── tools/
│   ├── visualize/              # 棋譜フレーム描画 + 学習曲線 ( matplotlib)
│   ├── analyze/                # 棋譜統計 + 2 run 比較 ( pandas)
│   └── tb_converter/           # JSONL → TensorBoard ( tensorboardX)
├── CLAUDE.md
└── README.md
```

## ビルド・テスト・リント

```bash
# Rust ビルド
cargo build --workspace

# Rust テスト
cargo test --workspace

# 単一テスト
cargo test --workspace <test_name>

# Lint ( CI で警告ゼロ強制)
cargo clippy --all-targets -- -D warnings

# フォーマット
cargo fmt
cargo fmt --check

# ベンチマーク
cargo bench

# ベンチのコンパイル確認のみ
cargo bench --no-run

# Doc ( 警告 0 を目指す)
cargo doc --workspace --no-deps

# Python tools ( uv 必須)
uv sync --all-packages
uv run pytest
```

## コーディング規約

- Rust edition = "2024"，rust-version = "1.85" 以上
- `unsafe` は使用しない
- ライブラリ層では `expect()` / `unwrap()` を避け，`Result` を返す
- パブリック型・関数には doc コメント `///` を付与
- `cargo clippy --all-targets -- -D warnings` をクリーンに保つ
- 依存クレートはワークスペースの `[workspace.dependencies]` に集約し，各クレートで `serde.workspace = true` のように継承

Python ツール:
- `ruff` でフォーマット・リント
- 型ヒント付与 ( 全関数シグネチャ)
- `pytest` で smoke test を追加

## 主要設計判断 ( Phase 1〜5)

- **Hybrid 盤面**: 8×8 では `Bitboard8` ( `u64 × 2`) で高速化．$4 \times 4$ から $26 \times 26$ までは `GenericBoard` ( `Vec<Option<Color>>`)
- **Bitboard レイアウト**: `bit_index = row * 8 + col` ( 行 0 列 0 = bit 0)
- **8 方向シフト**: 列マスク ( A 列・H 列) で wrap-around を防ぐ．5 回反復で連鎖石マスクを構築
- **同値性保証**: 8×8 では `Bitboard8` と `GenericBoard` の合法手・石返しが完全一致することをプロパティテストで検証
- **Full snapshot history**: GameHistory は各手後の `GameState` をすべて保持．`step_forward` / `step_backward` / `jump_to(n)` を $O(1)$ で提供
- **Pass の自動化**: 合法手なし時は engine が自動 Pass．Player が合法手ありで Pass を返したらエラー
- **棋譜 I/O の中立表現**: `GameRecord` を中間形式とし，JSON / GGF Reader/Writer を切り替え可能
- **Evaluator trait** ( Phase 5): MCTS の visit count や NN policy 値を覗くための補助 trait．`Player::evaluator()` が `Option<&mut dyn Evaluator>` を返す ( デフォルトは `None`)．`MctsPlayer` のみ実装．TUI Observe overlay で利用
- **External engine 連携** ( Phase 5): GTP / ntest 2 種類のプロトコル．`std::process::Command` 同期 IO + 別スレッド + `mpsc::channel` で 1 手タイムアウトをソフト実装．`tokio` は使わない
- **進捗バー** ( Phase 5): `BatchConfig.progress: Option<Arc<dyn ProgressCallback>>` で各局完了時にコールバック．CLI 側で `indicatif::ProgressBar` をラップ

## Markdown ファイル規約

- 本リポジトリ ( コードリポジトリ) の `.md` には Claude Code 生成フッタは **付けない**
- README.md は **英語** で記述する ( OSS / GitHub 公開を想定)
- CLAUDE.md など内部向けの `.md` は日本語で構わない．日本語の句読点は `，` と `．` を使用 ( `、` `。` は不可)

## Git ワークフロー

- `git init` はユーザが行う ( Claude は実行しない)
- 自動コミットは行わない

## Phase 完了状況

設計書 §10 に従い段階的に実装した．

| Phase | 追加クレート / 機能 | 状態 |
|---|---|---|
| Phase 1 | `othello-core` | 完了 |
| Phase 2 | `othello-player`, `othello-engine`, `othello-io`, `othello-cli` ( play / simulate) | 完了 |
| Phase 3 | `othello-tui`，WTHOR 読込，JSONL ロガー，`convert` / `inspect` / `replay` | 完了 |
| Phase 4 | `othello-rl`, `othello-py`，`BatchRunner` + `selfplay`，MCTS | 完了 |
| Phase 5 | 外部エンジン連携 ( gtp / ntest)，`tools/` 可視化・分析・TensorBoard，indicatif 進捗バー，Evaluator overlay | 完了 |
