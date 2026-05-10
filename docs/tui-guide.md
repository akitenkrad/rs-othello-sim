[English](tui-guide.md) | [日本語](ja/tui-guide.md)

# TUI Guide

`othello-tui` is a [ratatui](https://ratatui.rs/) frontend launched by
the `othello-cli` `play`, `replay`, and `observe` subcommands. It has
three distinct modes:

| Mode | Subcommand | Purpose |
|---|---|---|
| Play | `othello-cli play` (TUI variant) | Two humans take turns at the same terminal |
| Replay | `othello-cli replay --file …` | Step through a saved record |
| Observe | `othello-cli observe …` | Watch two AIs play, with an Evaluator overlay |

The Observe mode is unique in that it shows real-time evaluator
information when the side-to-move's player implements `Evaluator`
(currently MCTS and `nn:` variants).

## Layout

All three modes share the same skeleton:

```
+------------------+----------------------+
|                  | Players              |
|                  | Black: …             |
|     Board        | White: …             |
|     (centred)    +----------------------+
|                  | Status / Help        |
|                  | (mode-specific lines)|
|                  +----------------------+
|                  | Evaluator (Observe)  |
|                  | move  visits  bar    |
|                  +----------------------+
+------------------+----------------------+
| Footer keymap                            |
+------------------------------------------+
```

The Board panel renders 1.5×1 cells (each cell is two columns wide so
stones look round). Black is `B`, White is `W`, and legal moves for the
current side-to-move are highlighted with a dot.

## Play mode

Used when `othello-cli play` is built with the TUI front-end. Both
players input moves at the same terminal, alternating turns.

| Key | Action |
|---|---|
| Arrow keys / `h j k l` | Move cursor |
| `Enter` / `Space` | Place stone at cursor |
| `p` | Pass (only valid when no legal move) |
| `q` / `Esc` | Quit |

The status bar shows the side-to-move, the legal-move count, and a
warning if the cursor is on an illegal cell.

## Replay mode

```bash
othello-cli replay --file game.json --format json
```

Loads the entire record into a `Replayer`; navigation is `O(1)` because
each ply stores a full snapshot.

| Key | Action |
|---|---|
| `→` / `l` | Step forward one ply |
| `←` / `h` | Step backward one ply |
| `g` | Jump to the start |
| `G` | Jump to the end |
| `0`–`9` | Buffered "go to move N" (typed digits) |
| `q` / `Esc` | Quit |

The header shows `move N / TOTAL` and the score. Records that include
metadata (player name, timestamp) display them in the right-hand
Players panel.

## Observe mode

```bash
othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

Launches both AIs and runs the game in the TUI. With `--auto-delay 0`
(default), each ply requires `Space` to advance; with `--auto-delay
500` the next ply is requested after 500 ms.

| Key | Action |
|---|---|
| `Space` | Advance one ply (manual mode) |
| `q` / `Esc` | Quit |

The **Evaluator overlay** appears below the Players panel when the
side-to-move's player exposes `Player::evaluator()`. For `MctsPlayer`
this surface is the normalised root visit counts captured at the end
of the most recent `select_move`; for `NnEvaluator` it is the
post-mask, normalised policy:

```
Evaluator (top 5)
  D5    0.42  ███████░░░░░
  E6    0.21  ████░░░░░░░░
  F4    0.19  ███░░░░░░░░░
  C5    0.10  ██░░░░░░░░░░
  pass  0.08  █░░░░░░░░░░░
```

Bars are normalised so the largest value fills the whole width. The
overlay updates only after `select_move` returns; nothing is shown
while the engine is mid-search.

## Snapshot tests

The TUI rendering is locked in by [insta](https://insta.rs/) snapshot
tests in `crates/othello-tui/tests/snapshot.rs`. Updating the visual
layout requires regenerating those snapshots with `cargo insta
review`.

## Demo GIFs

The repository expects three GIFs under `docs/assets/` that
demonstrate each mode:

- `tui-play.gif`
- `tui-replay.gif`
- `tui-observe.gif`

See [`docs/assets/README.md`](assets/README.md) for capture
instructions (macOS screen recording or asciinema + agg).
