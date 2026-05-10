# External Engines

`rs-othello-sim` can drive third-party Othello engines as players via
`ExternalEnginePlayer`. This document explains:

1. Why we support external engines.
2. The two supported wire protocols (`gtp` and `ntest`).
3. How to integrate engines via the `external:` `PlayerSpec` syntax.
4. How to fetch and build real engines (Edax, Egaroucid) locally with
   `scripts/fetch_engines.sh`.
5. The license caveats that prevent us from bundling those binaries.
6. How to run the gated real-engine smoke tests.

## Why External Engines?

Built-in players (`random`, `greedy`, `mcts`) cover most simulation needs,
but research workflows occasionally require strong, well-known reference
opponents:

- **Calibration**: measuring how the in-tree MCTS player compares against
  Edax 4.4 at a fixed level.
- **Self-play data quality**: bootstrapping NN evaluators (Phase 6.4) with
  positions labeled by Edax / Egaroucid score.
- **Cross-implementation regression checks**: confirming that the in-tree
  rule engine agrees with established programs.

`ExternalEnginePlayer` spawns the engine as a subprocess, communicates
synchronously over stdin / stdout, and enforces a per-move timeout via a
worker thread + `mpsc::channel` (no `tokio` dependency).

## Mock vs Real Engines

Two layers of testing exist:

| Layer | Where | Runs in CI? | Purpose |
|---|---|---|---|
| Mock engines (bash) | `crates/othello-player/tests/mock_engine_*.sh` + `external_engine.rs` | Yes | Protocol-level conformance, fast |
| Real engines (Edax / Egaroucid) | `crates/othello-player/tests/real_engine.rs` | No (`#[ignore]`) | Smoke test against a real implementation |

The mock scripts implement the minimum surface of each protocol so that
the integration test can verify our request / response framing without
needing native binaries. Real binaries are fetched on demand by the user.

## `external:` PlayerSpec Grammar

```
external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,arg=VAL,...][,command=PATH]
```

| Key | Required | Default | Description |
|---|---|---|---|
| (positional) | One of positional or `command=` | - | Path to engine executable |
| `command` | One of positional or `command=` | - | Same as positional, but key=value form |
| `protocol` | No | `gtp` | `gtp` or `ntest` (aliases: `edax`, `egaroucid` -> `ntest`) |
| `timeout` | No | `30` | Per-move timeout in seconds |
| `arg` | No (repeatable) | - | Additional CLI argument forwarded to the engine |

Examples:

```text
external:/usr/local/bin/edax
external:./engines/edax,protocol=ntest,timeout=10
external:./engines/egaroucid,protocol=gtp,arg=--level,arg=1
external:command=/opt/edax/bin/lEdax-x64-modern,protocol=ntest
```

Use it just like any other player spec on the CLI:

```bash
cargo run -p othello-cli -- simulate \
  --black "external:./vendor/engines/edax/bin/lEdax-x64-modern,protocol=ntest" \
  --white "mcts:200,seed=1" \
  --board-size 8
```

## Protocols

### GTP (Go Text Protocol style, default)

Roughly:

```
> boardsize 8
< =
> clear_board
< =
> play black D3
< =
> genmove white
< = D5
> quit
< =
```

Each request is on its own line; responses begin with `=` (success) or
`?` (error). `pass` is a valid coordinate token.

### Ntest (Edax / Egaroucid simplified)

Roughly:

```
> set game <board_str>
< OK
> go
< D5
> quit
```

Edax in `--ggs` mode and Egaroucid's console mode both accept variations
on this. Adapter code lives in
`crates/othello-player/src/external/ntest.rs`.

## Fetching Real Engines

`scripts/fetch_engines.sh` downloads and (for Edax) builds engine binaries
into `vendor/engines/`. The directory is `.gitignore`d.

```bash
# Edax 4.4 (built from source via git clone + make)
bash scripts/fetch_engines.sh edax

# Egaroucid (prebuilt release tarball; URL must be edited in the script)
bash scripts/fetch_engines.sh egaroucid

# Both
bash scripts/fetch_engines.sh all

# Force reinstall (deletes existing vendor/engines/<name>/)
bash scripts/fetch_engines.sh all --force
```

The script auto-detects:

- `Darwin-arm64` -> Edax `ARCH=arm BUILD=osx`
- `Darwin-x86_64` -> Edax `ARCH=x64-modern BUILD=osx`
- `Linux-x86_64` -> Edax `ARCH=x64-modern BUILD=linux`
- `Linux-aarch64` -> Edax `ARCH=arm BUILD=linux`

Each engine directory ends up with a `LICENSE-NOTE.md` describing the
upstream source and license.

### Egaroucid Release URL Notes

Egaroucid release archive names change every version, so the script
contains `<TODO: fill release URL>` placeholders for each platform. Before
running `bash scripts/fetch_engines.sh egaroucid`, edit
`scripts/fetch_engines.sh` and replace the placeholder for your platform
with a concrete URL from
<https://github.com/Nyanyan/Egaroucid/releases>.

### Edax `eval.dat`

Edax requires an evaluation file (`eval.dat`) at runtime. It is
distributed separately from the source on
<https://www.abulmo.perso.neuf.fr/edax/4.4/> /
<https://eukaryote31.github.io/edax/> and is not bundled with the
GitHub source repository. After running the script, manually download
`eval.dat` and place it under `vendor/engines/edax/data/eval.dat` so
that Edax can find it relative to the binary.

## License Caveats

We deliberately do not redistribute Edax or Egaroucid binaries:

- **Edax**: GPL-2.0. Source is open, but `eval.dat` is proprietary; the
  user must obtain it separately.
- **Egaroucid**: GPL-3.0. Author-published release binaries can be
  downloaded but should not be re-hosted by third-party projects.

`scripts/fetch_engines.sh` is a *fetch helper* that runs locally on the
user's machine. It produces files under `vendor/engines/`, which is in
`.gitignore`, so nothing engine-specific is ever committed.

## Running the Gated Smoke Tests

The real-engine integration tests live in
`crates/othello-player/tests/real_engine.rs`. Every test there has
`#[ignore = "..."]`, so plain `cargo test` skips them entirely.

```bash
# 1. Fetch the engines (one-time):
bash scripts/fetch_engines.sh all

# 2. Run the smoke tests:
cargo test -p othello-player --test real_engine -- --ignored
```

If only one of the two engines is available, the corresponding tests
print `[skip] ... not installed` and pass without exercising anything,
so partial installations are fine.

What each test verifies:

| Test | Requires | Verifies |
|---|---|---|
| `edax_returns_a_legal_move` | `vendor/engines/edax/bin/...` | Edax responds to the opening position with a legal move |
| `egaroucid_returns_a_legal_move` | `vendor/engines/egaroucid/...` | Egaroucid responds to the opening position with a legal move |
| `edax_vs_egaroucid_short_match` | both | Six plies of Edax (Black, ntest) vs Egaroucid (White, gtp) without protocol errors or illegal moves |

The cross-engine match runs only six plies and does not assert on the
outcome; it is a smoke test for protocol interoperability, not a
strength benchmark.
