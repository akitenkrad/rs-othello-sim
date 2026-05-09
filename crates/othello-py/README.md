# othello-py

Python bindings for [`rs-othello-sim`](../..) built with [PyO3](https://pyo3.rs/) and packaged via [maturin](https://www.maturin.rs/).

## Build

The Python module is named `othello_sim`. Building requires Python >= 3.11.

```bash
# Inside this directory
cd crates/othello-py

# Install maturin if needed
pip install --user maturin

# Build and install into the active virtualenv
maturin develop --release
```

If your local Python is newer than the maximum version PyO3 0.22 officially supports (3.13), set the forward-compatibility flag before building:

```bash
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

`cargo build --workspace` succeeds without the `extension-module` feature (the Python interpreter is not linked in that mode), so CI does not require a Python toolchain.

## Usage

### Single-agent (Gymnasium-style)

```python
import othello_sim

env = othello_sim.OthelloEnv(
    board_size=8,
    opponent="random:seed=1",      # "random[:seed=N]" | "greedy" | "mcts:N[,seed=M]"
    observation_type="planes",     # "planes" | "flat" | "move_sequence"
    reward_mode="sparse",          # "sparse" | "dense"
    seed=42,
    agent_color="black",           # "black" | "white"
)
obs, info = env.reset(seed=42)
# obs is np.ndarray, dtype=float32
# info is a dict: {"action_mask": np.bool_, "legal_count": int, "move_number": int, "side_to_move": str}

obs, reward, terminated, truncated, info = env.step(action)
```

### Multi-agent (PettingZoo-style AECEnv)

```python
from othello_sim import OthelloMultiEnv

env = OthelloMultiEnv(board_size=8, observation_type="planes")
env.reset(seed=42)
while True:
    agent = env.current_agent()         # "black" or "white"
    mask = env.action_mask(agent)
    action = int(np.flatnonzero(mask)[0])
    obs, rew, term, trunc, info = env.step(action)
    if all(term.values()):
        break
```

A complete smoke test lives in [`examples/python_smoke.py`](examples/python_smoke.py).

## Action space

For an `H x W` board the discrete action space has size `H * W + 1`. Indices `0..H*W` map to board cells (`row * W + col`); the final index represents `Pass`.

## Observation shapes

| `observation_type` | dtype  | shape                |
|--------------------|--------|----------------------|
| `planes`           | float32| `(3, H, W)`          |
| `flat`             | float32| `(3 * H * W,)`       |
| `move_sequence`    | uint32 | `(L,)` where `L` = number of plies played |

For `planes` / `flat` the channels are `[own_stones, opp_stones, legal_mask]`; the legal mask is all zeros if it is not the agent's turn. `move_sequence` encodes plies as `1 + row * W + col` for placements and `0` for passes (Othello-GPT convention).
