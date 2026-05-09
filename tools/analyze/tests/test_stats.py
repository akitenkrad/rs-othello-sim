"""Smoke tests for analyze package."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from analyze.compare import chi_square_2x2
from analyze.load import load_records
from analyze.stats import compute_stats


def _record(winner: str, first_move: tuple[int, int], moves_count: int = 5) -> dict:
    moves = [
        {
            "n": i + 1,
            "side": "Black" if i % 2 == 0 else "White",
            "move": {
                "Place": {
                    "row": first_move[0] if i == 0 else 2,
                    "col": first_move[1] if i == 0 else 4,
                }
            },
            "ts": "2026-05-09T15:30:00.000+09:00",
        }
        for i in range(moves_count)
    ]
    return {
        "schema_version": "1.0",
        "metadata": {
            "id": f"id-{winner}-{first_move}",
            "started_at": "2026-05-09T15:30:00.000+09:00",
            "ended_at": "2026-05-09T15:30:42.000+09:00",
            "board_size": {"rows": 8, "cols": 8},
            "players": {
                "black": {"name": "Mcts", "params": {}},
                "white": {"name": "Random", "params": {}},
            },
            "result": {
                "winner": winner if winner != "Draw" else None,
                "score": {"black": 32, "white": 32},
            },
            "engine_version": "test",
        },
        "moves": moves,
    }


def test_load_records_and_compute_stats() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        records = [
            _record("Black", (2, 3), 6),
            _record("Black", (2, 3), 8),
            _record("White", (4, 5), 7),
            _record("Draw", (2, 3), 5),
        ]
        for i, r in enumerate(records):
            (tmp_path / f"game_{i}.json").write_text(json.dumps(r), encoding="utf-8")

        df = load_records(tmp_path)
        assert len(df) == 4

        stats = compute_stats(df)
        assert stats["games"] == 4
        assert stats["black_wins"] == 2
        assert stats["white_wins"] == 1
        assert stats["draws"] == 1
        # 平均手数は 6.5
        assert abs(stats["avg_moves"] - 6.5) < 1e-6
        # 初手 d3 ( 2,3 → "d3") が 3/4，f5 ( 4,5 → "f5") が 1/4
        dist = stats["first_move_distribution"]
        assert abs(dist.get("d3", 0.0) - 0.75) < 1e-6
        assert abs(dist.get("f5", 0.0) - 0.25) < 1e-6


def test_chi_square_basic() -> None:
    # 完全に同じなら chi^2 = 0
    r = chi_square_2x2(50, 100, 50, 100)
    assert r["statistic"] == 0.0
    # 大きく違えば chi^2 > 0
    r2 = chi_square_2x2(80, 100, 20, 100)
    assert r2["statistic"] > 10.0
    # p は (0, 1) の範囲
    assert 0.0 < r2["p_approx"] < 1.0
