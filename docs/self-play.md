[English](self-play.md) | [日本語](ja/self-play.md)

# Self-play & batch runs

`othello-cli selfplay` orchestrates many games in parallel using
`othello-engine::BatchRunner`, which wraps Rayon. Use it for
benchmarking strategies, generating training data for a neural-network
evaluator, and any "run N games and aggregate" workflow.

## Quick start

```bash
othello-cli selfplay \
  --num-games 1000 \
  --threads 8 \
  --black mcts:200 \
  --white random:seed=1 \
  --seed 42 \
  --log-dir auto \
  --swap-colors
```

| Flag | Default | Description |
|---|---|---|
| `--num-games N` | 100 | Total games to run |
| `--threads N` | 0 (auto) | 0 means "number of logical cores" |
| `--black SPEC` / `--white SPEC` | `random` | Player specs (see [cli-usage.md](cli-usage.md#playerspec-grammar)) |
| `--seed N` | _none_ | Each game gets `seed + game_index` |
| `--log-dir DIR` or `auto` | _none_ | Output directory; `auto` chooses `runs/selfplay_YYYYMMDD_HHMMSS/` |
| `--jsonl-log PATH` | _none_ | Aggregate JSONL log path (often `runs/.../all.jsonl`) |
| `--save-records FORMAT` | `json` | Per-game record format under `--log-dir` |
| `--swap-colors` | off | Swap Black/White on odd game indices |
| `--max-moves N` | _none_ | Safety bound; abort if a game exceeds N plies |
| `--no-progress` | off | Suppress the `indicatif` progress bar |

## Recommended layout

`--log-dir auto` produces a self-describing tree:

```
runs/
└── selfplay_20260510_120000/
    ├── game_0000.json
    ├── game_0001.json
    ├── …
    └── all.jsonl          # aggregate JSONL log
```

`game_*.json` are full `GameRecord`s (see
[record-formats.md](record-formats.md)) and `all.jsonl` is the JSONL
event stream emitted by `othello-io::JsonlLogger`. Both can be
processed by the Python toolchain in
[tools-visualize.md](tools-visualize.md).

## `--swap-colors`

Color matters in Othello; many heuristic players have a bias against
playing White. `--swap-colors` mitigates that by playing the same pair
of strategies twice (once with each side as Black) so the aggregate
win-rate is symmetric:

- Game 0, 2, 4, … : `--black SPEC_A`, `--white SPEC_B`
- Game 1, 3, 5, … : `--black SPEC_B`, `--white SPEC_A`

Half of the games are rendered as one assignment, half as the other,
which removes the color-asymmetric noise from the head-to-head metric.

## Progress bar

When stderr is a TTY, `indicatif::ProgressBar` shows a per-game
progress meter:

```
[00:00:42] [#######################-----------] 723/1000 (72%) ETA 16s
```

The bar receives one update per finished game via the
`ProgressCallback` trait wired into `BatchConfig.progress`. Library
users can implement that trait themselves for non-CLI integrations.

## Large-scale considerations

- **External engines** (`external:` SPECs) spawn one subprocess per
  game. Running 8 threads × 1000 games means up to 8 simultaneous
  engine processes (each game holds its own pair). Watch
  per-engine RSS and any per-process file-descriptor limits;
  `ulimit -n` and `ulimit -u` are easy to hit before the OOM killer
  notices. See [external-engines.md](external-engines.md).
- **NN evaluators** (`nn:` SPECs) load the model in each spawned
  player. For Candle on CPU the cost is small but non-zero; if you
  notice excessive overhead, prefer a single-process driver until
  `BatchRunner` learns to share an `Arc<Model>` (planned).
- **Determinism**. Each game's RNG seed is `seed + game_index`. For a
  fully reproducible batch, fix `--seed` and use deterministic player
  specs (`random:seed=…`, `mcts:N,seed=M`, `nn:…,deterministic`).

## Output analysis

Once a batch finishes, the analysis tools can summarise it:

```bash
# Per-game CSV statistics (win rate, average move count, opening dist.)
uv run analyze-stats --input runs/selfplay_20260510_120000/ --output stats.csv

# Two-batch comparison with chi-square / McNemar tests
uv run analyze-compare --a runs/run_a --b runs/run_b

# JSONL → TensorBoard scalars
uv run jsonl-to-tb --input runs/selfplay_20260510_120000/all.jsonl \
                   --output runs/tb/
tensorboard --logdir runs/tb/
```

See [tools-visualize.md](tools-visualize.md) for full coverage of the
Python toolchain.

## Self-play training loop (sketch)

`selfplay` is one half of an AlphaZero-style training loop. The other
half is the [replay buffer](replay-buffer.md) + a trainer (Python or
Rust). The combined flow:

```text
loop:
    # 1. Generate fresh games using the current model.
    othello-cli selfplay \
        --num-games N \
        --black "nn:safetensors:checkpoints/latest.safetensors" \
        --white "nn:safetensors:checkpoints/latest.safetensors" \
        --log-dir auto

    # 2. Convert the freshly produced JSON records into transitions.
    python trainer.py --records runs/selfplay_<ts>/ \
                      --buffer  buffers/per.bin

    # 3. Train one or more epochs on samples drawn from the buffer.
    python trainer.py --train --buffer buffers/per.bin \
                              --out checkpoints/next.safetensors

    # 4. Promote checkpoint and repeat.
    cp checkpoints/next.safetensors checkpoints/latest.safetensors
```

The Python wrapper around the buffer (see [replay-buffer.md](replay-buffer.md))
exposes `push`, `sample`, and `update_priorities` with numpy interop;
the `nn:` player spec consumes the resulting weights. Combine with
[nn-evaluator.md](nn-evaluator.md) for the IO schema and supported
formats.
