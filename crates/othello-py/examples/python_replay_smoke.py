"""Smoke test for the othello_sim ReplayBuffer Python bindings.

Build with::

    cd crates/othello-py
    maturin develop --release

Then run::

    python examples/python_replay_smoke.py

The script exercises both uniform and prioritized backends on a small set
of synthetic transitions so it runs in well under a second.
"""
from __future__ import annotations

import numpy as np

import othello_sim


def make_dummy_transition(board_size: int = 8, action: int = 0, value: float = 0.0):
    obs = np.zeros((3, board_size, board_size), dtype=np.float32)
    legal_mask = np.zeros(board_size * board_size + 1, dtype=bool)
    return obs, legal_mask, action, value


def run_uniform() -> None:
    print("=== Uniform replay buffer ===")
    rb = othello_sim.ReplayBuffer(capacity=64, kind="uniform", seed=0)
    print("kind:", rb.kind, "capacity:", rb.capacity, "len:", len(rb))
    for i in range(20):
        obs, mask, action, value = make_dummy_transition(action=i, value=float(i))
        rb.push(
            observation=obs,
            action=action,
            value=value,
            legal_mask=mask,
            side="Black",
            move_number=i,
            game_id="g0",
        )
    assert len(rb) == 20
    batch = rb.sample(batch_size=8)
    print("batch keys:", sorted(batch.keys()))
    print("observations:", batch["observations"].shape, batch["observations"].dtype)
    print("actions:", batch["actions"].shape, batch["actions"].dtype)
    print("values:", batch["values"].shape, batch["values"].dtype)
    print("legal_masks:", batch["legal_masks"].shape, batch["legal_masks"].dtype)
    print("indices:", batch["indices"].shape)
    print("policies:", batch["policies"])
    print("weights:", batch["weights"])
    assert batch["weights"] is None  # uniform buffer has no IS weights
    rb.clear()
    assert rb.is_empty()


def run_prioritized() -> None:
    print("=== Prioritized replay buffer ===")
    rb = othello_sim.ReplayBuffer(
        capacity=64,
        kind="prioritized",
        alpha=0.6,
        beta=0.4,
        beta_increment=0.01,
        epsilon=1e-6,
        seed=42,
    )
    print("kind:", rb.kind, "beta:", rb.beta())
    for i in range(32):
        obs, mask, action, value = make_dummy_transition(action=i, value=float(i))
        rb.push(
            observation=obs,
            action=action,
            value=value,
            legal_mask=mask,
            side="White",
            move_number=i,
            game_id="g1",
            policy=np.full(65, 1.0 / 65, dtype=np.float32),
        )
    batch = rb.sample(batch_size=8)
    print("indices:", batch["indices"])
    print("weights:", batch["weights"])
    print("policies shape:", batch["policies"].shape)
    assert batch["weights"] is not None
    assert batch["weights"].shape == (8,)
    # update priorities with synthetic td errors
    td = np.abs(np.random.default_rng(0).standard_normal(8)).astype(np.float32)
    rb.update_priorities(batch["indices"].astype(np.uint64), td)
    print("post-update beta:", rb.beta())


if __name__ == "__main__":
    run_uniform()
    run_prioritized()
