# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## リポジトリ概要

`rs-othello-sim` は Othello (Reversi) のシミュレーション研究および強化学習実験のための Rust ワークスペースである．設計書は親 Obsidian vault の `設計書/Othello_シミュレータ設計書.md` を参照すること．

現在は **Phase 1 ( コアゲーム) + Phase 2 ( ゲームループ層)** が実装済 ( `crates/othello-core`, `crates/othello-player`, `crates/othello-io`, `crates/othello-engine`, `crates/othello-cli`)．Phase 3 以降のクレートは将来追加される．

## ワークスペース構成

```
rs-othello-sim/
├── Cargo.toml                  # workspace manifest + shared deps
├── crates/
│   ├── othello-core/           # Phase 1: 盤面・ルール・状態
│   │   ├── src/                # color/coord/mv/error/bitboard/generic_board/board/rules/state
│   │   ├── tests/              # known_games, property_tests
│   │   └── benches/            # legal_moves, self_play (criterion)
│   ├── othello-player/         # Phase 2: Player trait + Random/Greedy/Human
│   ├── othello-io/             # Phase 2: GameRecord + JSON / GGF
│   ├── othello-engine/         # Phase 2: GameEngine, GameHistory, Replayer
│   └── othello-cli/            # Phase 2: play / simulate サブコマンド
├── CLAUDE.md
└── README.md
```

## ビルド・テスト・リント

```bash
# ビルド
cargo build --workspace

# テスト
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
```

## コーディング規約

- Rust edition = "2024"，rust-version = "1.85" 以上
- `unsafe` は使用しない
- ライブラリ層では `expect()` / `unwrap()` を避け，`Result` を返す
- パブリック型・関数には doc コメント `///` を付与
- `cargo clippy --all-targets -- -D warnings` をクリーンに保つ
- 依存クレートはワークスペースの `[workspace.dependencies]` に集約し，各クレートで `serde.workspace = true` のように継承

## 主要設計判断 ( Phase 1〜2)

- **Hybrid 盤面**: 8×8 では `Bitboard8` ( `u64 × 2`) で高速化．$4 \times 4$ から $26 \times 26$ までは `GenericBoard` ( `Vec<Option<Color>>`)
- **Bitboard レイアウト**: `bit_index = row * 8 + col` ( 行 0 列 0 = bit 0)
- **8 方向シフト**: 列マスク ( A 列・H 列) で wrap-around を防ぐ．5 回反復で連鎖石マスクを構築
- **同値性保証**: 8×8 では `Bitboard8` と `GenericBoard` の合法手・石返しが完全一致することをプロパティテストで検証
- **Full snapshot history**: GameHistory は各手後の `GameState` をすべて保持．`step_forward` / `step_backward` / `jump_to(n)` を $O(1)$ で提供
- **Pass の自動化**: 合法手なし時は engine が自動 Pass．Player が合法手ありで Pass を返したらエラー
- **棋譜 I/O の中立表現**: `GameRecord` を中間形式とし，JSON / GGF Reader/Writer を切り替え可能

## Markdown ファイル規約

- 本リポジトリ ( コードリポジトリ) の `.md` には Claude Code 生成フッタは **付けない**
- README.md は **英語** で記述する ( OSS / GitHub 公開を想定)
- CLAUDE.md など内部向けの `.md` は日本語で構わない．日本語の句読点は `，` と `．` を使用 ( `、` `。` は不可)

## Git ワークフロー

- `git init` はユーザが行う ( Claude は実行しない)
- 自動コミットは行わない

## 将来の Phase で追加予定のクレート

設計書 §10 に従い段階的に実装する．

| Phase | 追加クレート / 機能 | 状態 |
|---|---|---|
| Phase 1 | `othello-core` | 完了 |
| Phase 2 | `othello-player`, `othello-engine`, `othello-io`, `othello-cli` ( play / simulate) | 完了 |
| Phase 3 | `othello-tui`，WTHOR 読込，JSONL ロガー，`convert` / `inspect` / `replay` | 未着手 |
| Phase 4 | `othello-rl`, `othello-py`，`BatchRunner` + `selfplay`，MCTS | 未着手 |
| Phase 5 | 外部エンジン連携 ( Edax/Egaroucid)，`tools/` 可視化 | 未着手 |
