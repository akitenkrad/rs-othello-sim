"""CLI: compute summary statistics from a directory of game records."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

import pandas as pd

from analyze.load import load_records


def compute_stats(df: pd.DataFrame) -> dict[str, Any]:
    """Return a dict of summary statistics computed from a record DataFrame."""
    if df.empty:
        return {"games": 0}
    games = len(df)
    black_wins = int((df["winner"] == "Black").sum())
    white_wins = int((df["winner"] == "White").sum())
    draws = int(df["winner"].isna().sum() + (df["winner"] == "").sum())
    avg_moves = float(df["moves_count"].mean())
    avg_pass = float(df["pass_count"].mean())
    first_move_dist = (
        df["first_move"].fillna("(none)").value_counts(normalize=True).to_dict()
    )
    avg_black_score = float(df["black_score"].mean())
    avg_white_score = float(df["white_score"].mean())
    return {
        "games": games,
        "black_wins": black_wins,
        "white_wins": white_wins,
        "draws": draws,
        "black_win_rate": black_wins / games if games else 0.0,
        "white_win_rate": white_wins / games if games else 0.0,
        "draw_rate": draws / games if games else 0.0,
        "avg_moves": avg_moves,
        "avg_pass": avg_pass,
        "avg_black_score": avg_black_score,
        "avg_white_score": avg_white_score,
        "first_move_distribution": first_move_dist,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Aggregate Othello game record statistics.")
    parser.add_argument(
        "--input", "-i", type=Path, required=True, help="Directory or file of JSON records"
    )
    parser.add_argument("--output", "-o", type=Path, default=None, help="Output CSV path")
    args = parser.parse_args(argv)

    df = load_records(args.input)
    if df.empty:
        print(f"no records found under {args.input}", file=sys.stderr)
        return 1
    stats = compute_stats(df)
    if args.output is not None:
        flat = {k: v for k, v in stats.items() if not isinstance(v, dict)}
        flat_df = pd.DataFrame([flat])
        first_dist = stats.get("first_move_distribution", {})
        first_df = pd.DataFrame(
            [{"first_move": k, "frequency": v} for k, v in first_dist.items()]
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("w", encoding="utf-8") as fh:
            fh.write("# summary\n")
            flat_df.to_csv(fh, index=False)
            fh.write("\n# first_move distribution\n")
            first_df.to_csv(fh, index=False)
        print(f"wrote {args.output}")
    else:
        print(f"games: {stats['games']}")
        print(f"black_wins: {stats['black_wins']} ({stats['black_win_rate']:.1%})")
        print(f"white_wins: {stats['white_wins']} ({stats['white_win_rate']:.1%})")
        print(f"draws: {stats['draws']} ({stats['draw_rate']:.1%})")
        print(f"avg_moves: {stats['avg_moves']:.1f}")
        print(f"avg_pass: {stats['avg_pass']:.2f}")
        print("first move distribution:")
        for mv, freq in sorted(
            stats.get("first_move_distribution", {}).items(),
            key=lambda x: -x[1],
        ):
            print(f"  {mv}: {freq:.2%}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
