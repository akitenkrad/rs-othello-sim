"""CLI: plot win-rate / move-count curves from JSONL logs.

Usage:
    plot-curve --input runs/selfplay_*.jsonl --output curve.png
    plot-curve --input runs/dir/ --output curve.png
"""

from __future__ import annotations

import argparse
import glob
import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Plot learning curves from JSONL logs.")
    parser.add_argument(
        "--input", "-i", nargs="+", required=True, help="JSONL files or directories"
    )
    parser.add_argument("--output", "-o", type=Path, required=True, help="Output PNG file")
    parser.add_argument("--window", type=int, default=50, help="Rolling window size for win rate")
    args = parser.parse_args(argv)

    files: list[Path] = []
    for spec in args.input:
        path = Path(spec)
        if path.is_dir():
            files.extend(path.glob("*.jsonl"))
        else:
            for matched in glob.glob(spec):
                files.append(Path(matched))
    if not files:
        print("no input files matched", file=sys.stderr)
        return 1

    games_per_file: list[list[dict]] = []
    for f in files:
        games_per_file.append(_load_games(f))

    total_games = sum(len(g) for g in games_per_file)
    if total_games == 0:
        print("no game_end events found", file=sys.stderr)
        return 1

    win_flags: list[int] = []
    move_counts: list[int] = []
    for batch in games_per_file:
        for g in batch:
            win_flags.append(1 if g.get("winner") == "Black" else 0)
            move_counts.append(int(g.get("moves_total", 0)))

    fig, axes = plt.subplots(2, 1, figsize=(8, 6))
    axes[0].plot(_rolling_mean(win_flags, args.window), label=f"Black win rate (window={args.window})")
    axes[0].set_ylabel("Black win rate")
    axes[0].set_ylim(0.0, 1.0)
    axes[0].legend()

    axes[1].plot(move_counts, label="Total moves per game", color="tab:orange")
    axes[1].set_xlabel("Game index")
    axes[1].set_ylabel("Moves")
    axes[1].legend()

    fig.tight_layout()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(args.output)
    print(f"wrote {args.output} ({total_games} games)")
    return 0


def _load_games(path: Path) -> list[dict]:
    games: list[dict] = []
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if obj.get("event") == "game_end":
                games.append(obj)
    return games


def _rolling_mean(values: list[int], window: int) -> list[float]:
    if window <= 1:
        return [float(v) for v in values]
    out: list[float] = []
    for i in range(len(values)):
        lo = max(0, i - window + 1)
        chunk = values[lo : i + 1]
        out.append(sum(chunk) / len(chunk))
    return out


if __name__ == "__main__":
    sys.exit(main())
