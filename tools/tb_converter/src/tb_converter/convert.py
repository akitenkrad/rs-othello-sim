"""CLI: convert rs-othello-sim JSONL logs into TensorBoard event files.

Usage:
    jsonl-to-tb --input runs/selfplay_*/all.jsonl --output runs/tb/
"""

from __future__ import annotations

import argparse
import glob
import json
import sys
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Convert JSONL logs into TensorBoard events.")
    parser.add_argument(
        "--input", "-i", nargs="+", required=True, help="JSONL files or glob patterns"
    )
    parser.add_argument("--output", "-o", type=Path, required=True, help="Output log dir for TB")
    parser.add_argument("--window", type=int, default=50, help="Rolling window for win rate")
    args = parser.parse_args(argv)

    # tensorboardX 経由で event file を書き出す ( PyTorch 不要)．
    from tensorboardX import SummaryWriter  # type: ignore

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

    args.output.mkdir(parents=True, exist_ok=True)
    writer = SummaryWriter(log_dir=str(args.output))

    step = 0
    win_history: list[int] = []
    for f in files:
        for game in _iter_games(f):
            winner = game.get("winner")
            stones = game.get("stones") or {}
            black_score = int(stones.get("black", 0))
            white_score = int(stones.get("white", 0))
            moves_total = int(game.get("moves_total", 0))
            score_diff = black_score - white_score
            win_history.append(1 if winner == "Black" else 0)
            window = win_history[-args.window :]
            win_rate = sum(window) / len(window)

            writer.add_scalar("train/black_win_rate", win_rate, step)
            writer.add_scalar("train/episode_length", moves_total, step)
            writer.add_scalar("train/score_diff", score_diff, step)
            step += 1
    writer.flush()
    writer.close()
    print(f"wrote {step} steps to {args.output}")
    return 0


def _iter_games(path: Path):
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
                yield obj


if __name__ == "__main__":
    sys.exit(main())
