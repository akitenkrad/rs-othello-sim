# tb_converter

Convert rs-othello-sim JSONL logs into TensorBoard event files.

## Install

Part of the `rs-othello-sim` uv workspace:

```bash
uv sync
```

## CLI

```bash
uv run jsonl-to-tb --input runs/selfplay_*/all.jsonl --output runs/tb/
tensorboard --logdir runs/tb/
```

Scalars exported:
- `train/black_win_rate` (rolling)
- `train/episode_length`
- `train/score_diff` (black - white)
