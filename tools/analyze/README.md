# analyze

Statistical analysis for rs-othello-sim game records (JSON / JSONL).

## Install

Part of the `rs-othello-sim` uv workspace:

```bash
uv sync
```

## CLI

### `analyze-stats`

Aggregates a directory of JSON game records.

```bash
uv run analyze-stats --input runs/selfplay_20260509/ --output stats.csv
```

Reports win rate, average move count, opening-move distribution, pass-count distribution.

### `analyze-compare`

Compares two run directories and reports significance (chi-square / McNemar).

```bash
uv run analyze-compare --a runs/run_a --b runs/run_b
```

## API

```python
from analyze.load import load_records, load_jsonl
df = load_records("runs/selfplay_20260509/")
print(df.describe())
```
