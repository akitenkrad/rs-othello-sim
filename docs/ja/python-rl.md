[English](../python-rl.md) | [日本語](python-rl.md)

# Python バインディング & RL

`crates/othello-py` は PyO3 を介して Python に Engine を公開します． API は [Gymnasium](https://gymnasium.farama.org/) と [PettingZoo](https://pettingzoo.farama.org/) の慣例に従っています．拡張モジュール名は `othello_sim` です．

バインディングは 3 つの面を持ちます:

- `OthelloEnv` — single-agent ( Gymnasium ) ラッパー．
- `OthelloMultiEnv` — multi-agent ( PettingZoo AECEnv ) ラッパー．
- `ReplayBuffer` — uniform / prioritized experience Replay buffer ( [replay-buffer.md](replay-buffer.md) を参照 ) ．

## インストール

`maturin develop` は Rust 拡張をビルドし，アクティブな Python 環境にリンクします．

```bash
cd crates/othello-py

# maturin を 1 回インストール
pip install --user maturin

# Python が PyO3 のサポート範囲より新しい場合の前方互換フラグ
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

Rust ワークスペース自体は Python を必要としません． `cargo build --workspace` は Python ツールチェイン無しで成功します．

## `OthelloEnv` — single-agent

学習側が固定の色を担当し，相手は固定スクリプト戦略という SB3 / RLlib スタイルの学習向けに設計されています．

```python
import othello_sim

env = othello_sim.OthelloEnv(
    board_size=8,
    opponent="random:seed=1",   # Player spec ( CLI と同じ文法 )
    observation_type="planes",  # "planes" | "flat" | "move_sequence"
    reward_mode="sparse",       # "sparse" | "dense"
    seed=42,
    agent_color="black",        # "black" | "white"
)

obs, info = env.reset(seed=42)
# obs.dtype == np.float32
# info: {"action_mask": np.bool_, "legal_count": int,
#        "move_number": int, "side_to_move": str}

while True:
    mask = info["action_mask"]
    action = int(np.flatnonzero(mask)[0])
    obs, reward, terminated, truncated, info = env.step(action)
    if terminated or truncated:
        break
```

### Observation の種類

| `observation_type` | dtype  | shape                       |
|--------------------|--------|-----------------------------|
| `planes`           | float32| `(3, H, W)`                 |
| `flat`             | float32| `(3 * H * W,)`              |
| `move_sequence`    | uint32 | `(L,)` — `L` は経過 ply 数  |

`planes` / `flat` のチャネルは `[own_stones, opp_stones, legal_mask]` です．エージェントの手番でないとき legal mask は全 0 になります ( 環境側で相手手を自動解決してから返します ) ．

`move_sequence` は Othello-GPT の慣例に従い，各 ply を `1 + row * W + col` ， パスは `0` で表します．Transformer 系列モデル向けです．

### Reward モード

| `reward_mode` | 各ステップ | 終了時 |
|---|---|---|
| `sparse` | `0` | エージェント視点で `+1 / 0 / -1` |
| `dense` | `Δ stones / max_stones` | `+1 / 0 / -1` |

### Action mask + MaskablePPO

action space サイズは `H * W + 1` ( パスは末尾 ) ．環境は `info` に `action_mask` を入れて返します． [`sb3-contrib MaskablePPO`](https://sb3-contrib.readthedocs.io/) と組み合わせる例:

```python
import gymnasium as gym
import numpy as np
import othello_sim
from sb3_contrib import MaskablePPO
from sb3_contrib.common.maskable.utils import get_action_masks

class OthelloGym(gym.Env):
    def __init__(self):
        self._env = othello_sim.OthelloEnv(
            board_size=8, opponent="greedy", observation_type="planes",
            reward_mode="sparse", seed=0, agent_color="black",
        )
        self.observation_space = gym.spaces.Box(0.0, 1.0, (3, 8, 8))
        self.action_space = gym.spaces.Discrete(65)

    def reset(self, *, seed=None, options=None):
        obs, info = self._env.reset(seed=seed)
        self._mask = info["action_mask"]
        return obs, info

    def step(self, action):
        obs, reward, term, trunc, info = self._env.step(int(action))
        self._mask = info["action_mask"]
        return obs, reward, term, trunc, info

    def action_masks(self):
        return self._mask

model = MaskablePPO("CnnPolicy", OthelloGym(), verbose=1)
model.learn(100_000)
```

## `OthelloMultiEnv` — PettingZoo

両プレイヤーを制御する self-play や multi-agent アルゴリズム向けです．

```python
from othello_sim import OthelloMultiEnv

env = OthelloMultiEnv(board_size=8, observation_type="planes")
env.reset(seed=42)

while True:
    agent = env.current_agent()       # "black" | "white"
    mask = env.action_mask(agent)
    obs = env.observe(agent)          # (3, 8, 8) float32
    action = int(np.flatnonzero(mask)[0])
    obs_d, rew_d, term_d, trunc_d, info = env.step(action)
    if all(term_d.values()):
        break
```

返り値の dict はエージェント名でキー付けされます． `obs_d` には今着手したエージェント側の次の observation ( 必要なら相手の自動パス処理後 ) が入ります．もう一方は `None` です．

## ReplayBuffer

Rust の Replay buffer に対する Python ラッパーは [replay-buffer.md](replay-buffer.md) で詳述しています．簡単な例:

```python
import numpy as np
import othello_sim

buf = othello_sim.ReplayBuffer(
    capacity=100_000, kind="prioritized",
    alpha=0.6, beta=0.4, beta_increment=1e-3, epsilon=1e-6, seed=0,
)

buf.push(
    observation=np.zeros((3, 8, 8), dtype=np.float32),
    action=20, value=1.0,
    legal_mask=np.zeros(65, dtype=bool),
    side="Black", move_number=12, game_id="ep-0001",
    policy=np.full(65, 1.0 / 65, dtype=np.float32),  # optional
)

batch = buf.sample(batch_size=64)
# dict: observations / actions / values / legal_masks / policies
#       indices / weights

td = compute_td_errors(batch)                           # numpy float32
buf.update_priorities(batch["indices"], np.abs(td))     # PER のみ
```

## AlphaZero 風学習 ( スケッチ )

完全なループは multi-agent 環境，Replay buffer，NN evaluator，外部 trainer を組み合わせます．最小限のスケッチ:

```python
import numpy as np
import othello_sim

env = othello_sim.OthelloMultiEnv(board_size=8)
buf = othello_sim.ReplayBuffer(capacity=200_000, kind="uniform", seed=0)

for episode in range(num_episodes):
    env.reset(seed=episode)
    trajectory = []

    while True:
        agent = env.current_agent()
        mask = env.action_mask(agent)
        obs = env.observe(agent)
        # 自分のモデルで MCTS を走らせ，visit-count target pi
        # ( length H*W+1 ) と選択した action を返す．
        pi, action = run_mcts(obs, mask, model)
        trajectory.append((obs, action, mask, agent, pi))
        _, rew_d, term_d, _, _ = env.step(int(action))
        if all(term_d.values()):
            break

    z = rew_d   # 各サイドの最終報酬
    for i, (obs, action, mask, agent, pi) in enumerate(trajectory):
        buf.push(
            observation=obs, action=int(action),
            value=float(z[agent]),
            legal_mask=mask,
            side=agent.capitalize(),
            move_number=i,
            game_id=f"ep-{episode}",
            policy=pi.astype(np.float32),
        )

    if len(buf) >= warmup:
        batch = buf.sample(256)
        train_step(model, batch)
        # PER:
        # buf.update_priorities(batch["indices"], np.abs(td_errors))
```

ここでの `model` は自前のもの ( PyTorch / JAX など ) で，学習部分はユーザに委ねられます．checkpoint を Rust Engine 側にデプロイするには，IO スキーマに合致する safetensors または ONNX を出力し， `nn:` PlayerSpec から消費します — [nn-evaluator.md](nn-evaluator.md) を参照してください．

## 関連項目

- [Replay buffer](replay-buffer.md) — 完全な API ， sum-tree ， PER の詳細．
- [NN Evaluator](nn-evaluator.md) — `othello-cli` でロードできるよう Trainer が一致させるべき IO スキーマ．
- [Self-play & バッチ実行](self-play.md) — Python に触らずに学習データを生成する方法．
- `crates/othello-py/examples/` — 動作する smoke テスト群．
