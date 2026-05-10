[English](cli-usage.md) | [日本語](ja/cli-usage.md)

# CLI Usage

`othello-cli` is the single binary that drives every non-TUI workflow:
human-vs-human play, single simulated games, batch self-play, record
conversion and inspection, micro-benchmarks, and the Observe TUI for
watching AIs play.

This page is a recipe-style tour. For more focused guides see:

- [Self-play & batch runs](self-play.md) for the `selfplay` subcommand.
- [TUI guide](tui-guide.md) for Play / Replay / Observe modes.
- [Record formats](record-formats.md) for the file formats touched by
  `simulate`, `convert`, and `inspect`.
- [External engines](external-engines.md) for the `external:` player
  spec.
- [NN evaluator](nn-evaluator.md) for the `nn:` player spec.

## Global flags

These options apply to every subcommand.

| Flag | Default | Description |
|---|---|---|
| `--log-level <LEVEL>` | `info` | `trace` / `debug` / `info` / `warn` / `error` |
| `--log-format <FORMAT>` | `text` | `text` (human-readable) or `json` (structured) |
| `--log-file <PATH>` | _none_ | Additionally write JSON-formatted tracing events to `PATH` |

Logs go to stderr; redirect with shell `2>` if you want them in a
file. `--log-file` writes JSON regardless of `--log-format`.

## `play` — Human vs Human over stdin/stdout

```bash
othello-cli play --board-size 8
```

Prompts each side in turn. Enter coordinates as `d3`, `c4`, etc. Useful
for quickly verifying the build; for an interactive UI use
[`replay`](#replay--tui-record-playback) or [`observe`](#observe--tui-ai-vs-ai).

## `simulate` — One scripted game

```bash
othello-cli simulate \
  --board-size 8 \
  --black random:seed=42 \
  --white greedy \
  --save-record game.json \
  --record-format json
```

| Flag | Description |
|---|---|
| `--board-size N` | 4..=26, default 8 |
| `--black SPEC` / `--white SPEC` | Player spec (see [grammar](#playerspec-grammar)) |
| `--save-record PATH` | Optional output path for the game record |
| `--record-format json\|ggf` | Record format when saving |
| `--jsonl-log PATH` | Append JSONL events (`game_start`, `move`, `pass`, `game_end`) |

The game runs in-process. Use [`selfplay`](self-play.md) for batched
runs.

## `replay` — TUI record playback

```bash
othello-cli replay --file game.json --format json
othello-cli replay --file game.ggf  --format ggf

# Start in auto-play with a 250 ms-per-move tempo
othello-cli replay --file game.json --format json --auto --auto-delay 250
```

Loads the record into the TUI Replay screen with `O(1)` step-forward,
step-backward, and jump-to-N. WTHOR can be loaded after first
converting it via [`convert`](#convert--record-format-conversion).

| Flag | Effect |
|---|---|
| `--auto` | Begin in auto-play mode (otherwise wait for `Space` / `a`) |
| `--auto-delay MS` | Per-move delay in milliseconds (default `500`, clamped to `[50, 5000]`). Adjustable in-mode with `+` / `-`. |

Inside the Replay screen, `Space` or `a` toggles auto-play, `+` / `-`
adjusts the delay by 100 ms, `0` / `$` jump to start / end, and `<-` /
`->` (or `h` / `l`) step manually. See
[TUI guide](tui-guide.md#replay-mode) for the full key map.

## `convert` — Record format conversion

```bash
# Single record
othello-cli convert \
  --input game.wtb --input-format wthor \
  --output-format json --output game.json

# Bulk conversion (one game per file)
othello-cli convert \
  --input archive.wtb --input-format wthor \
  --output-format ggf --output-dir converted/

# JSON to GGF
othello-cli convert \
  --input game.json --input-format json \
  --output-format ggf --output game.ggf
```

| Flag | Description |
|---|---|
| `--input PATH` | Input file |
| `--input-format json\|ggf\|wthor` | Read format |
| `--output-format json\|ggf` | Write format (WTHOR is read-only) |
| `--output PATH` | Single-record output (mutually exclusive with `--output-dir`) |
| `--output-dir DIR` | Bulk output directory; one file per record |

WTHOR archives may contain many games; use `--output-dir` to expand
each into its own file. Format details live in
[record-formats.md](record-formats.md).

## `inspect` — Record statistics

```bash
# Single JSON / GGF file
othello-cli inspect --file game.json --format json

# WTHOR aggregate (totals across all games in the archive)
othello-cli inspect --file archive.wtb --format wthor
```

Reports board size, move count, winner, score, and (for WTHOR) total
games and per-result counts. Useful for sanity-checking a converted
archive before piping the output through [analyze tools](tools-visualize.md).

## `selfplay` — Batch self-play

The detailed walkthrough is in [self-play.md](self-play.md). Minimal
example:

```bash
othello-cli selfplay \
  --num-games 1000 --threads 8 \
  --black mcts:200 --white random:seed=1 \
  --seed 42 --log-dir auto --swap-colors
```

Highlights:

- `--log-dir auto` writes per-game JSON records and `all.jsonl` into
  `runs/selfplay_YYYYMMDD_HHMMSS/`.
- `--threads 0` (default) spans all logical cores via Rayon.
- A live `indicatif` progress bar is shown when stderr is a TTY; pass
  `--no-progress` to disable.
- `--swap-colors` alternates Black/White between odd and even games.

## `benchmark` — Micro-benchmarks

```bash
othello-cli benchmark --target legal-moves --duration 5
othello-cli benchmark --target self-play --board-size 8 --duration 5
othello-cli benchmark --target self-play --board-size 16 --duration 5
```

Reports ops/sec or moves/sec averaged over `--duration` seconds. For
criterion-based statistical benchmarks see
[benchmarks.md](benchmarks.md).

## `observe` — TUI AI vs AI

```bash
othello-cli observe --black mcts:500 --white greedy --auto-delay 500
```

Launches Observe mode (see [TUI guide](tui-guide.md)) with an
Evaluator overlay if the side-to-move's player implements `Evaluator`
(MCTS and `nn:` variants do; Random and Greedy do not).

| Flag | Description |
|---|---|
| `--auto-delay MS` | `0` waits for `Space` between moves; `>0` auto-advances |
| `--seed N` | Optional seed for stochastic players |

## PlayerSpec grammar

`--black` and `--white` accept the following grammar:

```
random[:seed=N]
greedy
mcts:N[,c=F][,seed=M][,depth=D][,tree_reuse=true]
external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,...][,command=PATH]
nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
```

| Spec | Notes |
|---|---|
| `random` | Uniform random over legal moves |
| `random:seed=N` | Deterministic RNG seed |
| `greedy` | Maximises immediate flips, ties broken by coordinate |
| `mcts:N` | UCT MCTS with `N` simulations per move |
| `mcts:N,c=F` | UCT exploration constant (default `sqrt(2)`) |
| `mcts:N,seed=M` | Deterministic rollouts |
| `mcts:N,depth=D` | Cap rollout depth |
| `mcts:N,tree_reuse=true` | Reuse subtree across moves (Phase 6.2) |
| `external:PATH` | Subprocess engine; see [external-engines.md](external-engines.md) |
| `nn:safetensors:PATH` | Candle-loaded NN evaluator (Phase 6.4) |
| `nn:onnx:PATH` | ONNX-loaded NN evaluator |

`temperature=F` and `deterministic` for `nn:` are documented in
[nn-evaluator.md](nn-evaluator.md). The `human` player has no spec; it
is wired automatically by `play` mode.
