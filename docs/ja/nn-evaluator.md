[English](../nn-evaluator.md) | [日本語](nn-evaluator.md)

# NN Evaluator ( Phase 6.4 )

`rs-othello-sim` は `crates/othello-nn/` 配下にニューラルネットワーク方策・価値 evaluator を備えています．これは AlphaZero 風 self-play 実験の基盤となるもので，モデルは行動空間上のポリシーヘッドと勝率を推定するバリューヘッドを公開し，グルー型の `NnEvaluator` がモデルを既存の `Player` / `Evaluator` トレイトに接続することで，対局を駆動したり TUI overlay へデータを供給したりできます．

このドキュメントでは目的，IO スキーマ，対応フォーマット，CLI 連携，そして現状の制約をカバーします．

## なぜ別クレートにしたか

Candle は重い依存ツリー ( gemm ， half ， candle-core / candle-nn / candle-onnx ) を持ち込みます．他のすべてのクレートが依存する `othello-player` にこれを追加するのではなく，NN バックエンドを独立した `othello-nn` クレートに置きました．`othello-player` は `nn:` SPEC 文字列のパースに Candle を必要としないように薄い `PlayerSpec::Nn` バリアントだけを保持し，実際の構築は `othello-cli` に委譲します．

この分割により， `othello-player` と `othello-nn` の循環依存を回避できます ( `othello-nn` 自体が `Player` / `Evaluator` トレイトに依存するため ) ．

## IO スキーマ

モデルは `othello-rl` が生成する 3 チャネル observation と同じものを扱います:

- 入力: `(batch, 3, H, W)` `f32`
  - チャネル 0: 自分の石 ( *side-to-move* が石を持つマスを 1.0 )
  - チャネル 1: 相手の石
  - チャネル 2: 合法手マスク ( side-to-move が着手できるマスにフラグ )
- 出力:
  - `policy`: `(batch, H * W + 1)` のポリシー *logits* ( softmax は evaluator 側で適用．モデルは出力しない ) ．インデックス `H*W` は `Move::Pass` ．
  - `value`: `(batch,)` ( または `(batch, 1)` ) ． *side-to-move* 視点での tanh 空間勝率推定値 ( `[-1, 1]` ) ．

`NnEvaluator` は非合法手をマスクし，残りの確率を再正規化したうえで，argmax ( `deterministic` ) または温度付きサンプリングで着手を決めます．

## 対応する重みフォーマット

2 つのバックエンドを提供します:

| バックエンド   | ローダー                                     | 補足 |
|---------------|--------------------------------------------|-------|
| safetensors   | `CandleModel::from_safetensors(path, ...)` | Candle ネイティブのローダー．モデルは `candle_model.rs` で定義したアーキテクチャとパラメータ名を使う必要があります． |
| ONNX          | `OnnxModel::from_path(path, ...)`          | `candle_onnx::simple_eval` を使用．デフォルトのテンソル名は入力 `input` ，出力 `policy` と `value` ． `OnnxModel::with_io_names` で上書き可能． |

Phase 6.4 では *学習済み重みは同梱されません* ．保存済みモデルなしでテストするために， `CandleModel::random_init(BoardSize, seed, Device)` が同じアーキテクチャを Candle のデフォルト乱数初期化で構築します．統合テストはこれを使っています．

同梱されるアーキテクチャは小型の AlphaZero 風 ResNet です:

```
input (B, 3, H, W)
  → conv 3x3 (3 → 32) → BN → ReLU             ( stem )
  → 2 × ResidualBlock (32, 32)
  ├─ policy head: conv 1x1 (32 → 2) → BN → ReLU → Linear(2*H*W → H*W + 1)
  └─ value  head: conv 1x1 (32 → 1) → BN → ReLU → Linear(H*W → 64)
                                              → ReLU → Linear(64 → 1) → tanh
```

## `Evaluator` トレイトとの関係

`NnEvaluator` は `othello_player::Player` と `othello_player::Evaluator` の両方を実装します．`select_move` 呼び出しのたびに，手ごとの確率マップ ( `HashMap<Move, f32>` ) が `last_policy` にキャッシュされます．TUI Observe overlay は `Player::evaluator()` 経由でこれを参照し，MCTS の訪問数を読むときと全く同じ要領でデータを取得します．

トレイトの返り値は `0..=1` の "高いほどよい" セマンティクスなので，バリューヘッドは `Evaluator::evaluate` からは公開されません — 公開されるのはマスク後・正規化済みのポリシーだけです．

## CLI での使い方

PlayerSpec に 2 つの新しいバリアントが追加されています:

```
nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
```

例:

```bash
# ポリシーヘッドの argmax で決定論的にプレイ．
othello-cli simulate \
  --black "nn:safetensors:./model.safetensors,deterministic" \
  --white "random:seed=42"

# 温度 0.5 のサンプリングを 200-rollout MCTS と対戦．
othello-cli selfplay --num-games 100 \
  --black "nn:onnx:./policy.onnx,temperature=0.5,seed=7" \
  --white "mcts:200"

# TUI でポリシー overlay を表示しながら観戦．
othello-cli observe \
  --black "nn:safetensors:./model.safetensors" \
  --white "greedy" \
  --auto-delay 500
```

構築は `othello-cli::player_spec_with_nn::build_player` で行われ，これが Candle をリンクする唯一の場所です．他のクレートは Candle 非依存のままです．

## 現状の制約

- 学習済み重みは同梱されていません．ランダム初期化はスモークテストにしか使えず，ほぼ一様プレイになります．
- モデルアーキテクチャはコード内で固定 ( チャネル 32 ， residual block 2 個 ) です．学習パイプラインが整い次第，より柔軟な config が用意されます．
- ONNX 統合テストはデフォルト無効です．IO スキーマに準拠したフィクスチャファイルが必要なためです．ユニットテストはローダーのエラーパス ( ファイル無し ) のみを検査します．
- ポリシーヘッドを事前分布として用いる AlphaZero 風 MCTS は *まだ未実装* です．現状 evaluator はポリシーから直接サンプリングします． Phase 7 のタスクで `NnEvaluator` と `MctsPlayer` を組み合わせる予定です．
- CPU 専用です． Metal / CUDA バックエンドは Candle 経由で到達可能ですが，本ワークスペースでは feature 化されていません．

## ロードマップ

Phase 6.4 を起点とする下流タスク:

- Phase 6.7 Replay buffer: `NnEvaluator` と既存 `selfplay` runner が生成する self-play 軌跡を収集する．
- Phase 7 ( 計画中 ) 学習パイプライン: PyTorch 側の trainer が buffer を読み，同じアーキテクチャを学習し， `othello-cli` がロードできるよう safetensors ( Candle ネイティブ ) または ONNX ( 相互運用 ) で重みを書き出す．
