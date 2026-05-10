[English](tools-visualize.md) | [日本語](ja/tools-visualize.md)

# Tools (visualize / analyze / TB)

Three small Python packages live under `tools/` and form a `uv`
workspace. Each is published as a console script and is intended for
post-processing the records and JSONL logs produced by `othello-cli`.

| Package | Console scripts | Purpose |
|---|---|---|
| `tools/visualize` | `plot-game`, `plot-curve` | Render game frames or learning curves with matplotlib |
| `tools/analyze` | `analyze-stats`, `analyze-compare` | Pandas-based aggregation + run comparison |
| `tools/tb_converter` | `jsonl-to-tb` | JSONL → TensorBoard event files |

`uv sync --all-packages` installs all three at once.

## Setup

```bash
# From the repository root
uv sync --all-packages

# Quick smoke
uv run plot-game --help
```

## `plot-game` — Render a single record

```bash
# Per-move PNG frames into a directory
uv run plot-game --input game.json --output-dir frames/

# Animated GIF (one frame per ply)
uv run plot-game --input game.json --gif game.gif
```

Reads a JSON record, walks the move list, and renders each ply as a
matplotlib figure. The board renderer is exposed as
`visualize.board_render.render_board` for reuse in notebooks. Useful
for visually verifying a single game (e.g. sanity-checking a
WTHOR-converted record).

See [record-formats.md](record-formats.md) for the record schema this
tool consumes.

## `plot-curve` — Learning curve from JSONL

```bash
uv run plot-curve --input runs/selfplay_*/all.jsonl --output curve.png
```

Aggregates one or many JSONL logs and plots win-rate / move-count
curves. Each input file becomes one line on the chart so you can
compare runs.

## `analyze-stats` — Aggregate a run directory

```bash
uv run analyze-stats --input runs/selfplay_20260510_120000/ \
                     --output stats.csv
```

Reports:

- Win rate (Black / White / Draw).
- Average move count.
- Opening-move distribution.
- Pass-count distribution.

Operates on the per-game JSON records produced by
`selfplay --log-dir auto`. CSV output makes it easy to ingest into
Jupyter or a spreadsheet.

## `analyze-compare` — Two-run statistical comparison

```bash
uv run analyze-compare --a runs/run_a --b runs/run_b
```

Runs chi-square and McNemar tests on the win-rate distributions of
two batches. Output includes the contingency table, the test
statistic, the p-value, and a one-line conclusion.

## `jsonl-to-tb` — Stream JSONL into TensorBoard

```bash
uv run jsonl-to-tb --input runs/selfplay_20260510_120000/all.jsonl \
                   --output runs/tb/

tensorboard --logdir runs/tb/
```

Scalars exported:

- `train/black_win_rate` (rolling)
- `train/episode_length`
- `train/score_diff` (Black − White)

Use `tensorboard --logdir runs/tb/` to view; multiple input files
become separate runs in the TB UI, so you can compare configurations
across batches.

## Putting it together

Typical workflow:

```bash
# 1. Generate data.
othello-cli selfplay \
    --num-games 5000 --threads 8 \
    --black mcts:200 --white random:seed=1 \
    --seed 42 --log-dir auto --swap-colors

# 2. Quick stats.
uv run analyze-stats \
    --input runs/selfplay_20260510_120000/ \
    --output stats.csv

# 3. Compare against an earlier batch.
uv run analyze-compare \
    --a runs/selfplay_20260509_180000/ \
    --b runs/selfplay_20260510_120000/

# 4. Stream training scalars into TensorBoard.
uv run jsonl-to-tb \
    --input runs/selfplay_20260510_120000/all.jsonl \
    --output runs/tb/

# 5. Render a representative game as a GIF for a paper / talk.
uv run plot-game \
    --input runs/selfplay_20260510_120000/game_0042.json \
    --gif game_0042.gif
```

## See also

- [Self-play & batch runs](self-play.md) — produces the inputs that
  these tools consume.
- [Record formats](record-formats.md) — JSON / GGF / JSONL schemas.
- The per-package README files under `tools/visualize/`,
  `tools/analyze/`, `tools/tb_converter/` for the package-level
  install / API quick reference.
