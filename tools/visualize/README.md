# visualize

Game record and learning curve visualization for rs-othello-sim.

## Install

This package is part of the `rs-othello-sim` uv workspace. From the repository root:

```bash
uv sync
```

## CLI

### `plot-game`

Renders each move of a JSON game record as a PNG (or animated GIF).

```bash
uv run plot-game --input game.json --output-dir frames/
uv run plot-game --input game.json --gif game.gif
```

### `plot-curve`

Plots win-rate / move-count curves from JSONL logs.

```bash
uv run plot-curve --input runs/selfplay_*.jsonl --output curve.png
```

## API

```python
from visualize.board_render import render_board

fig = render_board({
    "rows": 8,
    "cols": 8,
    "cells": [...],  # list of "B" / "W" / "."
})
fig.savefig("board.png")
```
