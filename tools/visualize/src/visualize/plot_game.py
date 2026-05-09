"""CLI: render each move of a JSON game record as a PNG (or animated GIF).

Usage:
    plot-game --input game.json --output-dir frames/
    plot-game --input game.json --gif game.gif
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from visualize.board_render import board_state_from_record, render_board


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render Othello game record frames.")
    parser.add_argument("--input", "-i", type=Path, required=True, help="Input JSON game record")
    parser.add_argument(
        "--output-dir",
        "-o",
        type=Path,
        default=None,
        help="Output directory for PNG frames (default: ./frames/)",
    )
    parser.add_argument("--gif", type=Path, default=None, help="Output GIF path (single animated file)")
    parser.add_argument("--dpi", type=int, default=80, help="Figure DPI (default: 80)")
    args = parser.parse_args(argv)

    with args.input.open("r", encoding="utf-8") as fh:
        record: dict[str, Any] = json.load(fh)

    moves = record.get("moves", [])
    total_frames = len(moves) + 1

    if args.gif is not None:
        _write_gif(record, total_frames, args.gif, args.dpi)
        print(f"wrote {args.gif} ({total_frames} frames)")
        return 0

    out_dir: Path = args.output_dir if args.output_dir is not None else Path("frames")
    out_dir.mkdir(parents=True, exist_ok=True)
    for i in range(total_frames):
        state = board_state_from_record(record, i)
        title = f"Move {i}/{len(moves)}"
        fig = render_board(state, title=title)
        path = out_dir / f"frame_{i:04d}.png"
        fig.savefig(path, dpi=args.dpi)
    print(f"wrote {total_frames} frames to {out_dir}")
    return 0


def _write_gif(record: dict[str, Any], total_frames: int, out_path: Path, dpi: int) -> None:
    """Render frames in-memory and assemble into a single animated GIF via Pillow."""
    from io import BytesIO

    from PIL import Image

    frames: list[Image.Image] = []
    for i in range(total_frames):
        state = board_state_from_record(record, i)
        fig = render_board(state, title=f"Move {i}/{total_frames - 1}")
        buf = BytesIO()
        fig.savefig(buf, format="png", dpi=dpi)
        buf.seek(0)
        frames.append(Image.open(buf).convert("RGB"))
    if not frames:
        return
    out_path.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(
        out_path,
        save_all=True,
        append_images=frames[1:],
        duration=400,
        loop=0,
    )


if __name__ == "__main__":
    sys.exit(main())
