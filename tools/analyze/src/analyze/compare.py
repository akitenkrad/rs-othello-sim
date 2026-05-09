"""CLI: compare two run directories and report statistical significance."""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path
from typing import Any

from analyze.load import load_records


def chi_square_2x2(a_wins: int, a_total: int, b_wins: int, b_total: int) -> dict[str, Any]:
    """Pearson chi-square test for two proportions ( 2×2 table without continuity correction)．

    Manually implemented to avoid a `scipy` dependency. Returns ``{statistic, dof, p_approx}``.
    The `p_approx` is approximated via the chi-square 1-dof CDF complement using the
    error-function representation (`p = erfc(sqrt(x/2))`).
    """
    a_loss = a_total - a_wins
    b_loss = b_total - b_wins
    obs = [[a_wins, a_loss], [b_wins, b_loss]]
    row_sums = [a_total, b_total]
    col_sums = [a_wins + b_wins, a_loss + b_loss]
    grand = a_total + b_total
    if grand == 0:
        return {"statistic": 0.0, "dof": 1, "p_approx": 1.0}
    chi2 = 0.0
    for i in range(2):
        for j in range(2):
            exp = row_sums[i] * col_sums[j] / grand
            if exp > 0:
                chi2 += (obs[i][j] - exp) ** 2 / exp
    p = math.erfc(math.sqrt(chi2 / 2.0))
    return {"statistic": chi2, "dof": 1, "p_approx": p}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Compare two Othello run directories.")
    parser.add_argument("--a", type=Path, required=True, help="Run directory A")
    parser.add_argument("--b", type=Path, required=True, help="Run directory B")
    args = parser.parse_args(argv)

    df_a = load_records(args.a)
    df_b = load_records(args.b)
    if df_a.empty or df_b.empty:
        print("one of the directories is empty", file=sys.stderr)
        return 1
    a_total = len(df_a)
    b_total = len(df_b)
    a_black_wins = int((df_a["winner"] == "Black").sum())
    b_black_wins = int((df_b["winner"] == "Black").sum())

    stat = chi_square_2x2(a_black_wins, a_total, b_black_wins, b_total)
    print(f"Run A: {a_black_wins}/{a_total} black wins ({a_black_wins / a_total:.2%})")
    print(f"Run B: {b_black_wins}/{b_total} black wins ({b_black_wins / b_total:.2%})")
    print(
        f"Chi-square (1 dof): {stat['statistic']:.3f}  p≈{stat['p_approx']:.4f}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
