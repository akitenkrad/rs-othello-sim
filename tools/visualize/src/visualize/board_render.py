"""Board → matplotlib Figure helpers.

Used by `plot_game` and any external script that wants to draw an Othello board.
"""

from __future__ import annotations

from typing import Any

import matplotlib

# 非対話バックエンドを優先 ( CI / headless 環境向け)．
matplotlib.use("Agg")
import matplotlib.patches as patches  # noqa: E402
from matplotlib.figure import Figure  # noqa: E402


def board_state_from_record(record: dict[str, Any], move_index: int) -> dict[str, Any]:
    """JSON 棋譜の `record` から `move_index` 手目までを再生した盤面状態を返す．

    `move_index = 0` なら初期局面．`move_index = len(moves)` なら最終局面．

    返り値:
      ``{"rows": int, "cols": int, "cells": [["B"|"W"|"."]]}``
    """
    bs = record["metadata"]["board_size"]
    rows = int(bs["rows"])
    cols = int(bs["cols"])
    grid: list[list[str]] = [["." for _ in range(cols)] for _ in range(rows)]
    # 標準初期配置 ( 4×4 以上で対応)．
    cr, cc = rows // 2, cols // 2
    grid[cr - 1][cc - 1] = "W"
    grid[cr - 1][cc] = "B"
    grid[cr][cc - 1] = "B"
    grid[cr][cc] = "W"

    moves = record.get("moves", [])
    for entry in moves[:move_index]:
        side = entry["side"]  # "Black" / "White"
        mv = entry["move"]
        if "Place" in mv:
            r = int(mv["Place"]["row"])
            c = int(mv["Place"]["col"])
            _apply_place(grid, r, c, side, rows, cols)
        # Pass は盤面変化なし
    return {"rows": rows, "cols": cols, "cells": grid}


def _apply_place(grid: list[list[str]], row: int, col: int, side: str, rows: int, cols: int) -> None:
    own = "B" if side == "Black" else "W"
    opp = "W" if own == "B" else "B"
    grid[row][col] = own
    # 8 方向で連鎖石をひっくり返す．
    for dr in (-1, 0, 1):
        for dc in (-1, 0, 1):
            if dr == 0 and dc == 0:
                continue
            r, c = row + dr, col + dc
            run: list[tuple[int, int]] = []
            while 0 <= r < rows and 0 <= c < cols and grid[r][c] == opp:
                run.append((r, c))
                r += dr
                c += dc
            if (
                run
                and 0 <= r < rows
                and 0 <= c < cols
                and grid[r][c] == own
            ):
                for rr, cc in run:
                    grid[rr][cc] = own


def render_board(state: dict[str, Any], title: str | None = None) -> Figure:
    """盤面状態を `matplotlib.Figure` として描画して返す．

    パラメータ:
      state: ``{"rows", "cols", "cells"}`` 形式の dict
      title: 図のタイトル ( オプション)
    """
    rows = int(state["rows"])
    cols = int(state["cols"])
    cells = state["cells"]
    fig = Figure(figsize=(cols, rows))
    ax = fig.add_subplot(111)
    # 緑の盤面背景
    ax.add_patch(patches.Rectangle((0, 0), cols, rows, facecolor="#1f7a1f"))
    # グリッド
    for r in range(rows + 1):
        ax.plot([0, cols], [r, r], color="black", linewidth=0.8)
    for c in range(cols + 1):
        ax.plot([c, c], [0, rows], color="black", linewidth=0.8)
    # 石
    for r in range(rows):
        for c in range(cols):
            ch = cells[r][c]
            if ch in ("B", "W"):
                color = "black" if ch == "B" else "white"
                edge = "white" if ch == "B" else "black"
                cx = c + 0.5
                # 行は上から表示するため反転
                cy = (rows - 1 - r) + 0.5
                circle = patches.Circle(
                    (cx, cy), 0.4, facecolor=color, edgecolor=edge, linewidth=0.8
                )
                ax.add_patch(circle)
    ax.set_xlim(0, cols)
    ax.set_ylim(0, rows)
    ax.set_xticks([c + 0.5 for c in range(cols)])
    ax.set_xticklabels([chr(ord("a") + c) for c in range(cols)])
    ax.set_yticks([r + 0.5 for r in range(rows)])
    ax.set_yticklabels([str(rows - r) for r in range(rows)])
    ax.set_aspect("equal")
    if title:
        ax.set_title(title)
    fig.tight_layout()
    return fig
