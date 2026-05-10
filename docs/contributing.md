# Contributing

A short guide to the conventions used in this repository. See
[CHANGELOG.md](../CHANGELOG.md) for the actual phase-by-phase history,
and the design document
(`設計書/Othello_シミュレータ設計書.md` in the parent Obsidian vault)
for higher-level intent.

## Environment

- **Rust** 1.85+ (`rust-version` in `Cargo.toml` is 1.85; edition
  2024).
- **Python** 3.11+ for the `tools/` workspace and `othello-py`.
- **`uv`** for Python package management. `maturin` for the PyO3
  extension. The Rust workspace alone has no Python dependency.

```bash
# One-time setup
cargo build --workspace
uv sync --all-packages

# Optional: PyO3 bindings
cd crates/othello-py
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 maturin develop --release
```

## Branching & commits

Current operational practice: commit and push directly to `main`.
Long-lived feature branches are not maintained; phases are tracked
through commit messages and `CHANGELOG.md` entries instead. Keep each
commit focused (one cohesive change) and reference the phase in the
message body if relevant.

## Tests

Run the full Rust suite locally before pushing:

```bash
cargo test --workspace                       # 331 + 5 ignored as of Phase 6.7
cargo test --workspace -- --include-ignored  # only when external engines are wired up
```

The testing layers ([design §8.1–8.3](#)):

| Layer | Tool | Where |
|---|---|---|
| Unit | `cargo test` | per crate, `src/` modules |
| Integration | `cargo test` | `crates/*/tests/` |
| Property | `proptest` | `crates/othello-core/tests/property_tests.rs` |
| Snapshot | `insta` | `crates/othello-tui/tests/snapshot.rs` |
| Smoke (gated) | `#[ignore]` | `crates/othello-player/tests/real_engine.rs` (Edax / Egaroucid) |

Python tests:

```bash
uv run pytest                              # all tools/* + othello-py
uv run pytest tools/visualize/tests/       # individual package
```

When adding a new feature, add tests at the layer it belongs to. New
public types deserve unit tests; new commands deserve integration
tests; new file formats deserve round-trip property tests.

## Lint, format, doc

CI runs the strict variants; mirror them locally before committing:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo doc --workspace --no-deps           # warnings = failure
```

Python:

```bash
uv run ruff check tools/ crates/othello-py/
uv run ruff format --check tools/ crates/othello-py/
```

`unsafe` is not used anywhere in the workspace; introducing it
requires explicit justification.

## Adding a new feature — checklist

1. **Design**. Update `設計書/Othello_シミュレータ設計書.md` if the
   change touches an externally visible contract.
2. **Implementation**. Keep crate boundaries clean (see
   [architecture.md](architecture.md)). New heavyweight dependencies
   (Candle, Tokio, etc.) should sit behind their own crate so the rest
   of the workspace does not pay the compile cost.
3. **Tests**. Unit + integration; property if a new invariant
   appears; snapshot if you touch the TUI.
4. **Docs**. Update or add a file under `docs/` and cross-link it
   from the related guides (the [README](../README.md) only links to
   the top-level docs).
5. **CHANGELOG**. Append an `### Added` / `### Changed` / `### Fixed`
   entry under the current phase.
6. **Verify**:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test --workspace
   cargo doc --workspace --no-deps
   ```

## Phase conventions

Phases roughly correspond to milestones in the design document. Each
phase produces:

- A coherent set of CHANGELOG entries.
- Test coverage at the new surfaces.
- Documentation updates (often a new file under `docs/`).

Phase 6 sub-tasks (6.1, 6.2, …) are independent; pick whichever has
the highest current value. The design document's §10 lists them.

## Documentation responsibilities

- Touching a public CLI flag → update [`docs/cli-usage.md`](cli-usage.md).
- Adding a new player → update [`docs/cli-usage.md`](cli-usage.md) and
  the relevant deep-dive (`docs/external-engines.md`,
  `docs/nn-evaluator.md`, etc.).
- Adding a new file format → update
  [`docs/record-formats.md`](record-formats.md).
- Changing crate dependencies → update the mermaid in
  [`docs/architecture.md`](architecture.md).
- Anything user-facing → make sure it shows up in the
  [README](../README.md) docs index if it justifies its own page.

The root README is intentionally short (intro + install + quick start
+ docs index); resist the temptation to grow it. Detail belongs in
`docs/`.
