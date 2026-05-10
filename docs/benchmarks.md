# Benchmarks

`rs-othello-sim` ships two flavours of performance measurement:

1. **Inline micro-benchmarks** via `othello-cli benchmark` — a quick
   ops/sec readout with no statistical overhead. Good for spot
   checking after a refactor.
2. **Criterion benchmarks** under `benches/` — statistically rigorous
   measurements with reports under `target/criterion/`.

Both feed back into the design document's §9 throughput goals
(reproduced below).

## Design goals (§9)

| Metric | Target | Conditions |
|---|---|---|
| 8×8 random self-play | ≥ 10⁶ moves/sec (single thread) | M2 / Apple Silicon |
| 8×8 legal-move generation | ≥ 10⁷ ops/sec | Bitboard |
| 16×16 random self-play | ≥ 10⁴ moves/sec | Generic |
| Batch self-play (8×8, 10000 games) | ≤ 30 s | 8 threads |
| GGF record load | ≥ 1000 records/sec | — |

## Inline benchmark CLI

```bash
# 8×8 legal-move generation
othello-cli benchmark --target legal-moves --duration 5

# 8×8 random self-play
othello-cli benchmark --target self-play --board-size 8 --duration 5

# 16×16 random self-play
othello-cli benchmark --target self-play --board-size 16 --duration 5
```

Each subcommand reports total iterations, throughput, and (for
self-play) games-per-second.

### Recent local readouts

Measured on Apple Silicon (M-series) with the `release` profile.
Numbers vary with thermal state and background load; treat them as
order-of-magnitude indicators against the §9 targets, not as
contractual values.

| Target | Reading | §9 target | Status |
|---|---|---|---|
| `legal-moves` (8×8 Bitboard) | ~4.2 × 10⁷ ops/s | ≥ 10⁷ | ✓ ~4× headroom |
| `self-play` (8×8) | ~7.0 × 10⁶ moves/s | ≥ 10⁶ | ✓ ~7× headroom |
| `self-play` (16×16) | ~2.8 × 10⁵ moves/s | ≥ 10⁴ | ✓ comfortably above |

Re-run the CLI commands above to refresh these numbers on your
hardware.

## Criterion benchmarks

Two crates host criterion benches:

```
crates/othello-core/benches/
├── legal_moves.rs   # bitboard vs generic legal-move generation
└── self_play.rs     # full random self-play loop

crates/othello-player/benches/
└── mcts_tree_reuse.rs   # Phase 6.2 A/B: tree_reuse on / off
```

Run all of them:

```bash
# Compile-only check (CI-friendly, no measurements)
cargo bench --workspace --no-run

# Full statistical run, reports under target/criterion/
cargo bench --workspace
```

The HTML reports include time series, distribution plots, and
comparison-against-baseline diffs.

### Viewing reports

```bash
# After `cargo bench`, open in a browser (relative path):
open target/criterion/report/index.html

# Per-bench reports:
open target/criterion/legal_moves/report/index.html
open target/criterion/self_play/report/index.html
open target/criterion/mcts_tree_reuse/report/index.html
```

The index page links to every group, summarising mean / median /
standard deviation and any regression vs the previous run.

### Important benches

| Bench | What it measures |
|---|---|
| `legal_moves` | Throughput of `Bitboard8` and `GenericBoard` legal-move generation across opening, midgame, and endgame positions |
| `self_play` | One full random vs random game; the inverse of the throughput is "moves/sec" |
| `mcts_tree_reuse` | Same MCTS configuration with `tree_reuse=false` vs `true`, on consecutive moves of the same game (Phase 6.2 A/B comparison) |

## Notes for contributors

- Criterion runs are **not** on CI. Local development relies on
  before/after comparisons; baseline labels are `cargo bench --
  --save-baseline NAME`, and `--baseline NAME` produces a regression
  report.
- Inline `othello-cli benchmark` is faster to run inside test loops
  but has no warm-up / statistical handling. Prefer it for "did my
  change move the needle?" questions and criterion for proper
  regression tracking.
- §9 targets assume the Bitboard fast path for 8×8 and the Generic
  path for everything else. Forcing `Board::Generic` for 8×8 is a
  useful experiment but will of course be slower than the targets.
- The full batch-self-play target (10 000 games, 8 threads, ≤ 30 s)
  is exercised manually with `othello-cli selfplay --num-games 10000
  --threads 8` rather than via criterion.

## See also

- [Architecture](architecture.md) for the hybrid board representation
  that the benches measure.
- [Self-play & batch runs](self-play.md) for end-to-end batch
  throughput.
- Design document §9 (`設計書/Othello_シミュレータ設計書.md`) for the
  long-form rationale behind each target.
