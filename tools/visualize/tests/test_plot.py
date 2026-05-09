"""Smoke tests for visualize package."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from visualize.board_render import board_state_from_record, render_board
from visualize.plot_curve import main as plot_curve_main
from visualize.plot_game import main as plot_game_main


def _sample_record() -> dict:
    return {
        "schema_version": "1.0",
        "metadata": {
            "id": "test-1",
            "started_at": "2026-05-09T15:30:00.000+09:00",
            "ended_at": "2026-05-09T15:30:42.000+09:00",
            "board_size": {"rows": 8, "cols": 8},
            "players": {
                "black": {"name": "Random", "params": {}},
                "white": {"name": "Random", "params": {}},
            },
            "result": {"winner": "Black", "score": {"black": 38, "white": 26}},
            "engine_version": "test",
        },
        "moves": [
            {
                "n": 1,
                "side": "Black",
                "move": {"Place": {"row": 2, "col": 3}},
                "ts": "2026-05-09T15:30:00.000+09:00",
            },
            {
                "n": 2,
                "side": "White",
                "move": {"Place": {"row": 2, "col": 2}},
                "ts": "2026-05-09T15:30:00.000+09:00",
            },
        ],
    }


def test_render_board_returns_figure() -> None:
    rec = _sample_record()
    state = board_state_from_record(rec, 0)
    fig = render_board(state)
    assert fig is not None
    assert state["rows"] == 8
    # 中央 4 マスに石が置かれている
    cells = state["cells"]
    assert cells[3][3] == "W"
    assert cells[4][4] == "W"
    assert cells[3][4] == "B"
    assert cells[4][3] == "B"


def test_board_state_after_one_move() -> None:
    rec = _sample_record()
    state = board_state_from_record(rec, 1)
    cells = state["cells"]
    # D3 = (2, 3) に黒
    assert cells[2][3] == "B"
    # 反転で D4 = (3, 3) も黒に
    assert cells[3][3] == "B"


def test_plot_game_writes_frames() -> None:
    rec = _sample_record()
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        rec_path = tmp_path / "g.json"
        rec_path.write_text(json.dumps(rec), encoding="utf-8")
        out_dir = tmp_path / "frames"
        rc = plot_game_main([
            "--input",
            str(rec_path),
            "--output-dir",
            str(out_dir),
            "--dpi",
            "60",
        ])
        assert rc == 0
        files = sorted(out_dir.glob("frame_*.png"))
        assert len(files) == 3  # 初期 + 2 手


def test_plot_curve_writes_png() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        log_path = tmp_path / "log.jsonl"
        with log_path.open("w", encoding="utf-8") as fh:
            for i in range(20):
                winner = "Black" if i % 2 == 0 else "White"
                fh.write(
                    json.dumps(
                        {
                            "event": "game_end",
                            "ts": "2026-05-09T15:30:00.000+09:00",
                            "game_id": f"id-{i}",
                            "winner": winner,
                            "stones": {"black": 32, "white": 32},
                            "moves_total": 60 - i,
                        }
                    )
                    + "\n"
                )
        out = tmp_path / "curve.png"
        rc = plot_curve_main([
            "--input",
            str(log_path),
            "--output",
            str(out),
            "--window",
            "5",
        ])
        assert rc == 0
        assert out.exists()
