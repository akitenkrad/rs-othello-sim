[English](../replay-buffer.md) | [日本語](replay-buffer.md)

# Replay Buffer

`othello-rl` には Rust 製 Replay buffer と numpy 互換の Python ラッパーが同梱されており，self-play 学習パイプライン ( DQN / double-DQN ， AlphaZero ， MuZero など ) のために設計されています．2 つのバックエンドは共通の [`ReplayBuffer`] トレイトを共有します:

| バックエンド                  | サンプリング   | 用途                                                            |
|------------------------------|---------------|----------------------------------------------------------------|
| `UniformReplayBuffer`        | 一様          | 素朴な Q-learning や behaviour-cloning ベースライン              |
| `PrioritizedReplayBuffer`    | 比例          | Prioritized Experience Replay ( Schaul et al., 2015 )           |

両者とも FIFO の循環バッファです．満杯になると最古の Transition が上書きされます．

## Transition スキーマ

記録される経験は [`Transition`] です:

| フィールド    | 型                            | 意味                                                                |
|---------------|-------------------------------|----------------------------------------------------------------|
| `observation` | `Array3<f32>` `(3, H, W)`     | `view` 側の planes エンコード ( `own / opp / legal_mask` )       |
| `action`      | `u32` ( `0..H*W+1` )          | セルインデックス．パスは `H*W`                                  |
| `policy`      | `Option<Array1<f32>>`         | AlphaZero 風 visit-count target ( 任意，長さ `H*W+1` )           |
| `value`       | `f32`                         | `view` 視点での終局報酬: `+1 / 0 / -1`                          |
| `legal_mask`  | `Array1<bool>` `(H*W+1,)`     | observation 時点の合法手マスク                                  |
| `side`        | `Color`                       | view プレイヤー                                                 |
| `move_number` | `u32`                         | 元の対局における手数インデックス                                |
| `game_id`     | `String`                      | 自由記述の識別子 ( デバッグ用途のみ )                           |

`value` は常に view プレイヤー視点で書き込まれます．Othello のスパース報酬 ( 勝者 / 敗者 / 引き分け ) はちょうど `+1 / -1 / 0` に対応します．TD 系の学習をするなら `value` には逆伝播済み TD ターゲットを書き込んでください．

## クイックスタート ( Rust )

```rust
use othello_rl::replay_buffer::{
    PrioritizedReplayBuffer, ReplayBuffer, UniformReplayBuffer,
    transitions_from_record_both_sides,
};
use othello_io::JsonReader;
use othello_io::GameRecordReader;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

let mut rng = ChaCha8Rng::seed_from_u64(0);
let mut buf = PrioritizedReplayBuffer::new(100_000)
    .with_alpha(0.6)
    .with_beta(0.4)
    .with_beta_increment(1e-3)
    .with_epsilon(1e-6);

// JSON 棋譜を side ごとの Transition に変換．
let record = JsonReader::new().read_one("game.json")?;
for t in transitions_from_record_both_sides(&record)? {
    buf.push(t);
}

// バッチをサンプリングし， TD 誤差で priority を更新．
let batch = buf.sample(64, &mut rng)?;
let td_errors: Vec<f32> = compute_td_errors(&batch);
buf.update_priorities(&batch.indices, &td_errors)?;
```

`UniformReplayBuffer` も同じトレイトを使いますが， `update_priorities` は無視します ( `Ok(())` を返すので汎用学習ループは分岐不要 ) ．

## Sum tree

`PrioritizedReplayBuffer` は [`SumTree`] に支えられています．完全二分木で，葉に priority ，内部ノードに部分和を持ちます． `total()` は `O(1)` ， `get(value)` は `O(log N)` で木を辿ります．バッチが累積優先度軸を均等にカバーするよう層化サンプリングを使います．

## Importance-sampling 重み

PER では `sample` が `weights = Some(w)` を返します．標準式は

```
w_i = (N * P(i))^(-beta) / max_j w_j
```

で，バッチ内の最大値で正規化することで最も優先度の高いサンプルの重みが `1.0` になります． `beta` はサンプリングのたびに `1.0` 方向へ増加します ( `with_beta_increment` / `with_beta` で設定 ) ．

## クイックスタート ( Python )

```python
import numpy as np
import othello_sim

buf = othello_sim.ReplayBuffer(
    capacity=100_000,
    kind="prioritized",          # or "uniform"
    alpha=0.6,
    beta=0.4,
    beta_increment=1e-3,
    epsilon=1e-6,
    seed=0,
)

obs = np.zeros((3, 8, 8), dtype=np.float32)
mask = np.zeros(65, dtype=bool)
buf.push(
    observation=obs,
    action=20,
    value=1.0,
    legal_mask=mask,
    side="Black",
    move_number=12,
    game_id="game-0001",
    policy=np.full(65, 1.0 / 65, dtype=np.float32),  # optional
)

batch = buf.sample(batch_size=64)
# batch は次のキーを持つ dict:
#   "observations" (B, 3, H, W) float32
#   "actions"      (B,)         uint32
#   "values"       (B,)         float32
#   "legal_masks"  (B, H*W+1)   bool
#   "policies"     (B, H*W+1)   float32 または None
#   "indices"      (B,)         uint64
#   "weights"      (B,)         float32 または None

# TD 誤差計算後， priority を更新 ( PER のみ ):
td = np.abs(td_errors).astype(np.float32)
buf.update_priorities(batch["indices"], td)
```

`buf.kind` は `"uniform"` または `"prioritized"` を返します． `buf.beta()` は PER では現在の `beta` を返し， uniform では `None` を返します．

## AlphaZero 風学習ループ ( スケッチ )

```python
import numpy as np
import othello_sim

env = othello_sim.OthelloMultiEnv(board_size=8)
buf = othello_sim.ReplayBuffer(capacity=200_000, kind="uniform", seed=0)

for episode in range(num_episodes):
    obs = env.reset(seed=episode)
    trajectory = []                       # (obs, action, mask, side)

    while True:
        agent = env.current_agent()
        mask = env.action_mask(agent)
        observation = env.observe(agent)  # (3, 8, 8) float32
        # ここで MCTS + ニューラルネット:
        pi, action = run_mcts(observation, mask)
        trajectory.append((observation, action, mask, agent, pi))
        obs_d, rew_d, term_d, trunc_d, info = env.step(int(action))
        if all(term_d.values()):
            break

    z = rew_d                             # 各 side の最終報酬
    for i, (observation, action, mask, agent, pi) in enumerate(trajectory):
        buf.push(
            observation=observation,
            action=int(action),
            value=float(z[agent]),
            legal_mask=mask,
            side=agent.capitalize(),
            move_number=i,
            game_id=f"episode-{episode}",
            policy=pi.astype(np.float32),
        )

    if len(buf) >= warmup:
        batch = buf.sample(256)
        train_step(batch)
```

PER を使う場合は， buffer を `kind="prioritized"` で構築し，各勾配ステップ後に `update_priorities(batch["indices"], td)` を呼び出すだけで OK です．

## 関連項目

- `crates/othello-rl/src/replay_buffer/` — Rust ソース
- `crates/othello-py/examples/python_replay_smoke.py` — 動作する例
- Schaul, T. *et al.* "Prioritized Experience Replay." *ICLR* 2016. arXiv:1511.05952
