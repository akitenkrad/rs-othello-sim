[English](../benchmarks.md) | [日本語](benchmarks.md)

# ベンチマーク

`rs-othello-sim` は 2 系統のパフォーマンス測定を提供します:

1. **インラインのマイクロベンチマーク** — `othello-cli benchmark` ．統計処理なしで ops/sec を素早く読むためのもの．リファクタリング後のスポットチェックに有用です．
2. **Criterion ベンチマーク** — `benches/` 配下．統計的に厳密な測定を行い， `target/criterion/` 以下にレポートを出力します．

両者は設計書 §9 のスループット目標 ( 以下に再掲 ) に紐付いています．

## 設計目標 ( §9 )

| 指標 | 目標 | 条件 |
|---|---|---|
| 8×8 random self-play | 単一スレッドで ≥ 10⁶ moves/sec | M2 / Apple Silicon |
| 8×8 合法手生成 | ≥ 10⁷ ops/sec | Bitboard |
| 16×16 random self-play | ≥ 10⁴ moves/sec | Generic |
| バッチ self-play ( 8×8 ， 10000 局 ) | ≤ 30 秒 | 8 threads |
| GGF 棋譜ロード | ≥ 1000 records/sec | — |

## インラインベンチマーク CLI

```bash
# 8×8 合法手生成
othello-cli benchmark --target legal-moves --duration 5

# 8×8 random self-play
othello-cli benchmark --target self-play --board-size 8 --duration 5

# 16×16 random self-play
othello-cli benchmark --target self-play --board-size 16 --duration 5
```

各サブコマンドは合計反復数，スループット，および ( self-play では ) games-per-second を表示します．

### 直近のローカル測定値

Apple Silicon ( M シリーズ ) ， `release` プロファイルで計測．数値は熱状態と他プロセスの負荷に依存するため，§9 目標との比較ではオーダの目安として扱い，契約値とはみなさないでください．

| Target | 測定値 | §9 目標 | 状態 |
|---|---|---|---|
| `legal-moves` ( 8×8 Bitboard ) | 約 4.2 × 10⁷ ops/s | ≥ 10⁷ | ✓ ヘッドルーム約 4 倍 |
| `self-play` ( 8×8 ) | 約 7.0 × 10⁶ moves/s | ≥ 10⁶ | ✓ ヘッドルーム約 7 倍 |
| `self-play` ( 16×16 ) | 約 2.8 × 10⁵ moves/s | ≥ 10⁴ | ✓ 余裕あり |

各自の環境では上記 CLI コマンドを再実行して数値を取り直してください．

## Criterion ベンチマーク

Criterion ベンチを持つクレートは 2 つです:

```
crates/othello-core/benches/
├── legal_moves.rs   # bitboard vs generic の合法手生成
└── self_play.rs     # 完全 random self-play ループ

crates/othello-player/benches/
└── mcts_tree_reuse.rs   # Phase 6.2 の A/B: tree_reuse on/off
```

すべて実行する場合:

```bash
# コンパイルチェックのみ ( CI フレンドリ ，計測なし )
cargo bench --workspace --no-run

# 統計付きフル実行．レポートは target/criterion/
cargo bench --workspace
```

HTML レポートには時系列，分布プロット，前回ベースラインに対する diff が含まれます．

### レポートの閲覧

```bash
# `cargo bench` の後にブラウザで開く ( 相対パス ):
open target/criterion/report/index.html

# 個別レポート:
open target/criterion/legal_moves/report/index.html
open target/criterion/self_play/report/index.html
open target/criterion/mcts_tree_reuse/report/index.html
```

index ページから各グループへリンクされ，平均 / 中央値 / 標準偏差，および前回比較が一覧できます．

### 重要なベンチ

| Bench | 計測内容 |
|---|---|
| `legal_moves` | 序盤・中盤・終盤の局面における `Bitboard8` と `GenericBoard` の合法手生成スループット |
| `self_play` | random vs random の完全 1 局．逆数が "moves/sec" |
| `mcts_tree_reuse` | 同じ MCTS 設定で `tree_reuse=false` と `true` を，同じ対局の連続する手で対比 ( Phase 6.2 の A/B 比較 ) |

## コントリビューター向けの注意

- Criterion 実行は CI には **入れていません** ．ローカル開発では before/after 比較に頼ります．ベースラインラベルは `cargo bench -- --save-baseline NAME` で，比較は `--baseline NAME` でリグレッションレポートを生成します．
- インラインの `othello-cli benchmark` はテストループ内で速く回せますが，ウォームアップや統計処理がありません．「変更で針が動いたか」の判断はインライン，正式なリグレッション追跡は criterion を使ってください．
- §9 の目標値は 8×8 では Bitboard 高速パス，それ以外は Generic パスを前提としています．8×8 で `Board::Generic` を強制するのは有用な実験ですが，目標値は当然下回ります．
- 完全なバッチ self-play 目標 ( 10 000 局， 8 threads ， ≤ 30 秒 ) は criterion ではなく `othello-cli selfplay --num-games 10000 --threads 8` で手動確認します．

## 関連項目

- [アーキテクチャ](architecture.md) — ベンチが計測する hybrid board 表現について．
- [Self-play & バッチ実行](self-play.md) — エンドツーエンドのバッチスループット．
- 設計書 §9 ( `設計書/Othello_シミュレータ設計書.md` ) — 各目標値の根拠．
