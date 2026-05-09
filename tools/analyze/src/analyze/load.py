"""Record / log loaders → pandas DataFrame.

Supported sources:
  - Directory of JSON game records ( `load_records`)
  - JSONL log file ( `load_jsonl`)
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pandas as pd


def load_records(path: str | Path) -> pd.DataFrame:
    """Load all `*.json` game records under `path` (file or directory).

    Returns a DataFrame with metadata + result columns:
      ``[id, board_rows, board_cols, black_name, white_name, winner,
         black_score, white_score, moves_count, pass_count, first_move]``
    """
    p = Path(path)
    files: list[Path]
    if p.is_dir():
        files = sorted(p.glob("*.json"))
    else:
        files = [p]
    rows: list[dict[str, Any]] = []
    for f in files:
        try:
            with f.open("r", encoding="utf-8") as fh:
                rec = json.load(fh)
        except (OSError, json.JSONDecodeError):
            continue
        rows.append(_record_to_row(rec))
    return pd.DataFrame(rows)


def _record_to_row(rec: dict[str, Any]) -> dict[str, Any]:
    md = rec.get("metadata", {})
    bs = md.get("board_size", {})
    players = md.get("players", {})
    result = md.get("result") or {}
    score = result.get("score") or {}
    moves = rec.get("moves", [])
    first_move = None
    for m in moves:
        mv = m.get("move", {})
        if "Place" in mv:
            r = int(mv["Place"]["row"])
            c = int(mv["Place"]["col"])
            first_move = f"{chr(ord('a') + c)}{r + 1}"
            break
    pass_count = sum(1 for m in moves if "Pass" in (m.get("move") or {}))
    return {
        "id": md.get("id"),
        "board_rows": bs.get("rows"),
        "board_cols": bs.get("cols"),
        "black_name": (players.get("black") or {}).get("name"),
        "white_name": (players.get("white") or {}).get("name"),
        "winner": result.get("winner"),
        "black_score": score.get("black"),
        "white_score": score.get("white"),
        "moves_count": len(moves),
        "pass_count": pass_count,
        "first_move": first_move,
    }


def load_jsonl(path: str | Path) -> pd.DataFrame:
    """Load a JSONL log into a DataFrame.

    Each row is one event. Columns vary per event ( `event`, `ts`, `game_id`, ...).
    """
    p = Path(path)
    rows: list[dict[str, Any]] = []
    with p.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            rows.append(obj)
    return pd.DataFrame(rows)
