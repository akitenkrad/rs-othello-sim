[English](external-data.md) | [日本語](ja/external-data.md)

# External data sources

This page surveys public sources of Othello game records and related
datasets, and shows how to bring them into `rs-othello-sim`. For the
on-disk schemas of each format see [Record formats](record-formats.md);
for the engine binaries used to validate or annotate those records see
[External engines](external-engines.md).

## Quick reference

| Source | Format | Volume | License | Importer |
|---|---|---|---|---|
| WTHOR (French Othello Federation) | `.wtb` binary | ~1977–present, hundreds of thousands of master games, updated yearly | Research / non-commercial; cite FFO | `othello-cli fetch wthor` then `convert --input-format wthor` |
| GGS / GGF archives | `.ggf` text | Computer & online matches, decades of archives | Archive-specific; check before redistribution | `othello-cli convert --input-format ggf` |
| Online play sites (eOthello / Othello Quest / GGS) | Per-site (often GGF or JSON) | Per-account or per-tournament | Per-site terms of service | After conversion to GGF or JSON |
| Self-generated (`selfplay`) | JSON + JSONL | Bounded only by compute | MIT (this repo) | Native |

## WTHOR — French Othello Federation

- Landing page: <https://www.ffothello.org/informatique/la-base-wthor/>
- Yearly archives are distributed as `wth_YYYY.wtb` files (sometimes inside `.zip`). Auxiliary `JOUEUR.JOU` and `TOURNOI.TOU` lookup tables map player / tournament IDs to names.
- Format details: see [`docs/record-formats.md` → WTHOR](record-formats.md). The header is 16 bytes, each game 68 bytes (8 bytes of metadata + 60 bytes of move codes).
- WTHOR encodes only `real_score` and a sequence of move bytes. Passes are *implicit* — when a player has no legal move the move byte simply skips. The reader in `othello-io::wthor` replays each game on a `Board::standard_8x8()` and inserts a `Move::Pass` when the side to move has no legal moves.
- License: free for research and non-commercial use; redistribution and academic use require crediting the Fédération Française d'Othello.

```bash
# 0. Use the bundled fetcher (recommended; auto-extracts and dedupes).
othello-cli fetch wthor --year 2023 --dest data/wthor/

# Or fetch a range of years in one go:
othello-cli fetch wthor --years 2020..2023 --dest data/wthor/

# 1. Fetch (browser or curl).
curl -L -o wth_2023.zip https://www.ffothello.org/wthor/wth_2023.zip
unzip wth_2023.zip

# 2. Inspect (per-file aggregates: counts, win rates, average length, year).
./target/release/othello-cli inspect --file wth_2023.wtb --format wthor

# 3. Convert into per-game JSON files (one game per file under data/wthor_2023/).
./target/release/othello-cli convert \
  --input wth_2023.wtb --input-format wthor \
  --output-format json --output-dir data/wthor_2023/

# 4. Replay the first game with auto-play.
./target/release/othello-cli replay \
  --file data/wthor_2023/game_00001.json --format json \
  --auto --auto-delay 200
```

## GGF — Generic Game Format

- Specification & archive index: <http://www.skatgame.net/mburo/ggsa/ggf>
- GGF is a textual node-tree format used by GGS (Generic Game Server) and several engine self-tests. The Othello subset implemented here covers the `GM` / `PB` / `PW` / `RE` / `BO` / `B[…]` / `W[…]` tags. Engine think times (`B[D3//1.234]`) parse but are dropped on write.
- Many community archives are distributed as concatenated multi-game `.ggf` text files. `othello-cli convert` and `inspect` handle both single and multi-game inputs.

```bash
# Convert a multi-game GGF archive into one JSON per game
./target/release/othello-cli convert \
  --input archive.ggf --input-format ggf \
  --output-format json --output-dir data/ggf_archive/
```

## Online play sites

These sites host Othello matches on the web and sometimes expose game records. Always check the current terms of service before bulk-downloading; *bulk scraping is generally not allowed* even when individual exports are permitted.

| Site | URL | Typical export |
|---|---|---|
| eOthello | <https://www.eothello.com/> | Per-game GGF / JSON via the user's profile |
| Othello Quest (WOC) | <https://www.othelloquest.com/> | Per-account export; community tooling exists |
| GGS (legacy) | Mirrored archives under the GGF index above | Bundled GGF archives |

Convert anything you obtain to GGF or JSON first, then feed it through the regular `convert` / `replay` / `inspect` pipeline.

## Synthetic and research datasets

Several Othello-AI papers ship — or describe how to regenerate — large datasets:

- **Othello-GPT** (Li et al., 2022, [arXiv:2210.13382](https://arxiv.org/abs/2210.13382)) trains on ~20 M synthetic 60-ply uniformly-random legal games. You can regenerate equivalent data with this repo:

  ```bash
  ./target/release/othello-cli selfplay \
    --board-size 8 --num-games 20000 \
    --black "random:seed=1" --white "random:seed=2" \
    --threads 8 --log-dir auto --save-records json
  ```

  The aggregated `runs/selfplay_*/all.jsonl` is convenient input for `tools/tb_converter` (see [tools-visualize.md](tools-visualize.md)).

- **OLIVAW** (Norelli & Panconesi, 2021, [arXiv:2103.17228](https://arxiv.org/abs/2103.17228)) trains AlphaZero-style purely from self-play; the dataset itself is not released, but the recipe maps directly onto `selfplay` with [`nn:safetensors:`](nn-evaluator.md) opponents and the [replay buffer](replay-buffer.md).

- **Edax / Egaroucid annotations** (see [external-engines.md](external-engines.md)) — pair any of the corpora above with engine evaluations to get annotated transitions for supervised baselines.

## Building a corpus from scratch

When research demands a *known* distribution you control end-to-end, generate locally:

```bash
# 1k games of MCTS(200) vs Random with color swap; per-game JSON + a single JSONL
./target/release/othello-cli selfplay \
  --board-size 8 --num-games 1000 --threads 8 \
  --black "mcts:200,seed=42" --white "random:seed=99" \
  --swap-colors --log-dir auto --save-records json

# Aggregate stats and a TensorBoard summary
uv run analyze-stats --input runs/selfplay_*/ --output runs/stats.csv
uv run jsonl-to-tb   --input runs/selfplay_*/all.jsonl --output runs/tb/
```

## Working with large corpora

- WTHOR's per-game files are tiny (~1 KB JSON each). A full year is comfortably under 1 GB.
- Use ramdisk or `--threads N` only when the bottleneck is CPU; for pure I/O conversion `--threads 1` is usually enough.
- For replay-buffer pipelines, prefer feeding `transitions_from_record_both_sides` directly (see [replay-buffer.md](replay-buffer.md)) instead of round-tripping through JSON.
- Always keep a copy of the original archive (`.wtb` / `.ggf`) for reproducibility; converted JSONs are derivatives.

## License & attribution checklist

- WTHOR — credit the **Fédération Française d'Othello**; cite the year of the archive.
- GGS / GGF archives — cite Michael Buro's GGS pages and any per-archive readme.
- Synthetic datasets — credit the originating paper (`Li et al. 2022`, `Norelli & Panconesi 2021`).
- Self-generated data — record the exact `selfplay` invocation (seed, version) in the experiment notes.

## See also

- [Record formats](record-formats.md) — on-disk schemas for JSON / GGF / WTHOR / JSONL.
- [CLI usage](cli-usage.md) — `convert`, `inspect`, `selfplay`, `replay`.
- [External engines](external-engines.md) — Edax / Egaroucid for evaluation and annotation.
- [Replay buffer](replay-buffer.md) — turning records into RL transitions.
