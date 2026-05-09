"""Smoke test for tb_converter."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest
from tb_converter.convert import main as convert_main


def test_convert_writes_event_file() -> None:
    pytest.importorskip("tensorboardX")
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        log_path = tmp_path / "log.jsonl"
        with log_path.open("w", encoding="utf-8") as fh:
            for i in range(5):
                fh.write(
                    json.dumps(
                        {
                            "event": "game_end",
                            "ts": "2026-05-09T15:30:00.000+09:00",
                            "game_id": f"id-{i}",
                            "winner": "Black" if i % 2 == 0 else "White",
                            "stones": {"black": 32 + i, "white": 32 - i},
                            "moves_total": 60 - i,
                        }
                    )
                    + "\n"
                )
        out_dir = tmp_path / "tb"
        rc = convert_main(["--input", str(log_path), "--output", str(out_dir), "--window", "3"])
        assert rc == 0
        assert out_dir.exists()
        # tensorboardX は events.out.tfevents.* を生成
        events = list(out_dir.glob("events.out.tfevents.*"))
        assert len(events) >= 1
