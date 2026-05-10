[English](replay-buffer.md) | [日本語](ja/replay-buffer.md)

# Replay Buffer

`othello-rl` ships a Rust replay buffer plus a numpy-friendly Python wrapper
designed for self-play training pipelines (DQN/double-DQN, AlphaZero, MuZero,
etc.). Two backends share a common [`ReplayBuffer`] trait:

| Backend                     | Sampling      | When to use                                                   |
|-----------------------------|---------------|---------------------------------------------------------------|
| `UniformReplayBuffer`       | Uniform       | Vanilla Q-learning / behaviour-cloning baselines              |
| `PrioritizedReplayBuffer`   | Proportional  | Prioritized Experience Replay (Schaul et al., 2015)           |

Both buffers are FIFO rings: once full, the oldest transition is overwritten.

## Transition schema

Every recorded experience is a [`Transition`]:

| Field         | Type                          | Meaning                                                        |
|---------------|-------------------------------|----------------------------------------------------------------|
| `observation` | `Array3<f32>` `(3, H, W)`     | Planes encoding (`own / opp / legal_mask`) from `view`'s side  |
| `action`      | `u32` in `0..H*W+1`           | Cell index, or `H*W` for `Pass`                                |
| `policy`      | `Option<Array1<f32>>`         | AlphaZero-style visit-count target (optional, length `H*W+1`)  |
| `value`       | `f32`                         | Terminal reward from `view`'s perspective: `+1 / 0 / -1`       |
| `legal_mask`  | `Array1<bool>` `(H*W+1,)`     | Legal-action mask at the time of the observation               |
| `side`        | `Color`                       | The view player                                                |
| `move_number` | `u32`                         | Move index within the source game                              |
| `game_id`     | `String`                      | Free-form identifier (debug only)                              |

The `value` is always written from the view player's perspective. Sparse Othello
rewards (winner / loser / draw) collapse to `+1 / -1 / 0` exactly. For TD-style
training, write the back-propagated TD target into `value` instead.

## Quick start (Rust)

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

// Convert a JSON game record into per-side transitions.
let record = JsonReader::new().read_one("game.json")?;
for t in transitions_from_record_both_sides(&record)? {
    buf.push(t);
}

// Sample a batch and update priorities after computing TD errors.
let batch = buf.sample(64, &mut rng)?;
let td_errors: Vec<f32> = compute_td_errors(&batch);
buf.update_priorities(&batch.indices, &td_errors)?;
```

`UniformReplayBuffer` uses the same trait but ignores `update_priorities`
(it returns `Ok(())` so generic training loops do not need to branch).

## Sum tree

`PrioritizedReplayBuffer` is backed by [`SumTree`], a complete binary tree with
priorities at the leaves and partial sums at the internal nodes. `total()`
is `O(1)` and `get(value)` walks the tree in `O(log N)`. Stratified sampling
is used so the batch covers the cumulative-priority axis evenly.

## Importance-sampling weights

For PER, `sample` returns `weights = Some(w)` with the standard formula

```
w_i = (N * P(i))^(-beta) / max_j w_j
```

normalised by the batch maximum so the highest-priority sample receives weight
`1.0`. `beta` increments toward `1.0` after every sample (configurable via
`with_beta_increment` / `with_beta`).

## Quick start (Python)

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
# batch is a dict with keys:
#   "observations" (B, 3, H, W) float32
#   "actions"      (B,)         uint32
#   "values"       (B,)         float32
#   "legal_masks"  (B, H*W+1)   bool
#   "policies"     (B, H*W+1)   float32 or None
#   "indices"      (B,)         uint64
#   "weights"      (B,)         float32 or None

# After computing TD errors, update priorities (PER only):
td = np.abs(td_errors).astype(np.float32)
buf.update_priorities(batch["indices"], td)
```

`buf.kind` returns `"uniform"` or `"prioritized"`. `buf.beta()` returns the
current `beta` for PER, or `None` for uniform buffers.

## AlphaZero-style training loop (sketch)

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
        # MCTS + neural network here:
        pi, action = run_mcts(observation, mask)
        trajectory.append((observation, action, mask, agent, pi))
        obs_d, rew_d, term_d, trunc_d, info = env.step(int(action))
        if all(term_d.values()):
            break

    z = rew_d                             # final terminal reward per side
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

For Prioritized Experience Replay, simply construct the buffer with
`kind="prioritized"` and call `update_priorities(batch["indices"], td)` after
each gradient step.

## See also

- `crates/othello-rl/src/replay_buffer/` — Rust source
- `crates/othello-py/examples/python_replay_smoke.py` — runnable example
- Schaul, T. *et al.* "Prioritized Experience Replay." *ICLR* 2016. arXiv:1511.05952
