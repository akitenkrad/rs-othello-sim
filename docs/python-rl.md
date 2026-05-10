# Python bindings & RL

`crates/othello-py` exposes the engine to Python through PyO3, with
APIs that follow the [Gymnasium](https://gymnasium.farama.org/) and
[PettingZoo](https://pettingzoo.farama.org/) conventions. The
extension module is named `othello_sim`.

The bindings cover three surfaces:

- `OthelloEnv` — single-agent (Gymnasium) wrapper.
- `OthelloMultiEnv` — multi-agent (PettingZoo AECEnv) wrapper.
- `ReplayBuffer` — uniform / prioritized experience replay buffer
  (see [replay-buffer.md](replay-buffer.md)).

## Install

`maturin develop` builds the Rust extension and links it into the
active Python environment.

```bash
cd crates/othello-py

# Install maturin once
pip install --user maturin

# Forward-compatibility flag if your Python is newer than PyO3 supports
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

The Rust workspace itself does not require Python; `cargo build
--workspace` succeeds without any Python toolchain.

## `OthelloEnv` — single-agent

Designed for SB3 / RLlib-style training where the learner controls
one fixed color and the opponent is a fixed scripted strategy.

```python
import othello_sim

env = othello_sim.OthelloEnv(
    board_size=8,
    opponent="random:seed=1",   # Player spec (same grammar as the CLI)
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

### Observation types

| `observation_type` | dtype  | shape                       |
|--------------------|--------|-----------------------------|
| `planes`           | float32| `(3, H, W)`                 |
| `flat`             | float32| `(3 * H * W,)`              |
| `move_sequence`    | uint32 | `(L,)` — `L` plies played   |

For `planes` / `flat` the channels are
`[own_stones, opp_stones, legal_mask]`. The legal mask is all zeros
when it is not the agent's turn (the env auto-resolves opponent
moves before returning).

`move_sequence` follows the Othello-GPT convention: each ply is
`1 + row * W + col`, `0` for passes. Useful for transformer-style
sequence models.

### Reward modes

| `reward_mode` | Per-step | Terminal |
|---|---|---|
| `sparse` | `0` | `+1 / 0 / -1` from agent's perspective |
| `dense` | `Δ stones / max_stones` | `+1 / 0 / -1` |

### Action mask + MaskablePPO

The action space size is `H * W + 1` (Pass is the last index). The
env returns an `action_mask` in `info`. To use it with
[`sb3-contrib MaskablePPO`](https://sb3-contrib.readthedocs.io/):

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

For self-play and multi-agent algorithms where both sides are
controlled.

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

Returned dicts are keyed by agent name. `obs_d` contains the
just-played agent's next observation (after the opponent's auto-pass
if applicable); the other entry is `None`.

## ReplayBuffer

The Python wrapper around the Rust replay buffer is documented in
detail in [replay-buffer.md](replay-buffer.md). Quick example:

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
buf.update_priorities(batch["indices"], np.abs(td))     # PER only
```

## AlphaZero-style training (sketch)

The full loop combines the multi-agent env, the replay buffer, the NN
evaluator, and an external trainer. A minimal sketch:

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
        # MCTS guided by your model. Returns visit-count target pi
        # (length H*W+1) and the chosen action.
        pi, action = run_mcts(obs, mask, model)
        trajectory.append((obs, action, mask, agent, pi))
        _, rew_d, term_d, _, _ = env.step(int(action))
        if all(term_d.values()):
            break

    z = rew_d   # final terminal reward per side
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

The `model` here is your own (PyTorch, JAX, …); training is left to
the user. To deploy a checkpoint back into the Rust engine, export
either safetensors or ONNX matching the IO schema and consume it via
the `nn:` PlayerSpec — see [nn-evaluator.md](nn-evaluator.md).

## See also

- [Replay buffer](replay-buffer.md) — full API, sum-tree, PER details.
- [NN evaluator](nn-evaluator.md) — the IO schema your trainer must
  match to be loadable by `othello-cli`.
- [Self-play & batch runs](self-play.md) — for generating training
  data without touching Python.
- `crates/othello-py/examples/` — runnable smoke tests.
