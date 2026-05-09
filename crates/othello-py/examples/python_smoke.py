"""Smoke test for the othello_sim Python bindings.

Build with::

    cd crates/othello-py
    maturin develop --release

Then run::

    python examples/python_smoke.py
"""
from __future__ import annotations

import numpy as np
import othello_sim


def run_single_agent() -> None:
    env = othello_sim.OthelloEnv(
        board_size=8,
        opponent="random:seed=1",
        observation_type="planes",
        reward_mode="sparse",
        seed=42,
        agent_color="black",
    )
    obs, info = env.reset(seed=42)
    print(f"obs shape: {obs.shape}")
    print(f"action_mask shape: {info['action_mask'].shape}")
    print(f"side_to_move: {info['side_to_move']}")
    print(f"action_space_size: {env.action_space_size}")

    total_reward = 0.0
    steps = 0
    while True:
        mask: np.ndarray = info["action_mask"]
        legal = np.flatnonzero(mask)
        if legal.size == 0:
            break
        action = int(legal[0])
        obs, reward, terminated, truncated, info = env.step(action)
        total_reward += reward
        steps += 1
        if terminated or truncated:
            print(f"Episode done after {steps} steps, total_reward={total_reward}")
            print(env.render())
            break


def run_multi_agent() -> None:
    env = othello_sim.OthelloMultiEnv(
        board_size=8,
        observation_type="flat",
        reward_mode="sparse",
    )
    obs = env.reset(seed=7)
    print("agents:", env.agents())
    while True:
        agent = env.current_agent()
        mask = env.action_mask(agent)
        legal = np.flatnonzero(mask)
        action = int(legal[0]) if legal.size else 64
        obs, rew, term, trunc, info = env.step(action)
        if all(term.values()):
            print("multi env done; rewards:", rew)
            break


if __name__ == "__main__":
    run_single_agent()
    run_multi_agent()
