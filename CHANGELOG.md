# Changelog

All notable changes to `rs-othello-sim` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Phase 6 (in progress)

### Added

- TUI Replay mode: tunable auto-play with `[Space]` / `[a]` toggle,
  `[+]` / `[-]` to adjust the per-move delay (100 ms steps, clamped to
  `[50, 5000]` ms). The delay is shown in the header as `[AUTO <ms>ms]`
  and auto-play stops automatically when the final ply is reached.
  Toggling auto-play at the end restarts from move 0. The CLI gains
  `othello-cli replay --auto --auto-delay <ms>` to start in auto mode,
  and `othello_tui::run_replay_with_options` exposes the same options
  to library users while keeping the existing `run_replay(history)`
  API as a thin wrapper.
- 6.1: `scripts/fetch_engines.sh` to fetch and build Edax / Egaroucid into `vendor/engines/`.
- 6.1: `crates/othello-player/tests/real_engine.rs` integration tests (gated behind `#[ignore]`,
  run with `cargo test -p othello-player --test real_engine -- --ignored`).
- 6.1: `docs/external-engines.md` with engine setup, license notes, and PlayerSpec reference.
- 6.1: `.gitignore` entry for `/vendor/engines/` so fetched binaries are not committed.
- 6.2: `MctsConfig.tree_reuse` flag and `MctsConfig::with_tree_reuse(bool)`. When
  enabled, MctsPlayer reuses the subtree corresponding to the agent's chosen move
  and the opponent's reply, amortising visit cost across moves. Default is `false`
  to preserve existing behaviour.
- 6.2: `MctsPlayer::last_root_visit_total()` accessor exposing the accumulated root
  visit count after each `select_move` (useful for tree-reuse diagnostics and tests).
- 6.2: `crates/othello-player/benches/mcts_tree_reuse.rs` criterion A/B benchmark
  (compile-only in CI; run locally with `cargo bench -p othello-player`).
- 6.2: `crates/othello-player/tests/mcts_tree_reuse.rs` integration tests covering
  full-game play with `tree_reuse=true` on 8x8 and 4x4 boards plus first-move
  parity with the baseline.
- 6.2: `docs/architecture.md` describing crate dependencies, hybrid board
  representation, MCTS internals (including tree-reuse), the `Evaluator` trait,
  and the `BatchRunner` / `ProgressCallback` integration.
- 6.4: New `othello-nn` crate with Candle-based NN evaluator. `NnEvaluator`
  implements both `Player` and `Evaluator` so it can drive games and feed the
  TUI overlay. `CandleModel::from_safetensors` and `OnnxModel::from_path`
  cover the two main weight formats; `CandleModel::random_init` is provided
  for tests since no trained weights ship in this phase.
- 6.4: `nn:safetensors:PATH[,temperature=F,deterministic,seed=N]` and
  `nn:onnx:PATH[,...]` PlayerSpec variants. Construction is delegated to
  `othello-cli::player_spec_with_nn::build_player` so `othello-player` does
  not depend on Candle.
- 6.4: `PlayerSpec::try_build_player` (fallible) and `SpecError::NeedsNnBackend`
  for callers that should not link Candle. `PlayerSpec::build_player` keeps
  its existing signature and panics on `Nn` variants with a clear message
  pointing at the CLI wrapper.
- 6.4: `othello_tui::run_observe_with_players` accepts pre-built
  `Box<dyn Player>` instances so the TUI can host NN players without taking
  a dependency on Candle.
- 6.4: `docs/nn-evaluator.md` describing the IO schema, supported formats,
  CLI usage, and the relation to the existing `Evaluator` trait.
- 6.7: Replay buffer in `othello-rl::replay_buffer`. `UniformReplayBuffer`
  and `PrioritizedReplayBuffer` (PER + SumTree) share a `ReplayBuffer`
  trait. `transitions_from_record[_both_sides]` constructs transitions
  from JSON / GGF / WTHOR game records. The Python `ReplayBuffer` class
  in `othello-py` exposes `push` / `sample` / `update_priorities` with
  numpy interop.
- 6.7: `docs/replay-buffer.md` documenting the Rust API, Python API,
  Transition schema, sum-tree internals, and an AlphaZero-style training
  loop sketch.
- 6.9: Reorganized documentation. The root README is compressed to a
  one-screen overview (intro, demo placeholder, install, quick start,
  docs index, license). New `docs/` files cover getting started,
  CLI usage, TUI guide, self-play, record formats, Python bindings,
  tools, benchmarks, and contributing. Existing architecture / external-
  engines / nn-evaluator / replay-buffer docs are cross-linked.
- 6.9: `docs/assets/` placeholder with capture instructions for the
  three TUI demo GIFs (tui-play, tui-replay, tui-observe).
- 6.9: Japanese translations of all 13 documentation files under
  `docs/ja/`, plus `README_ja.md`. Every documentation file (English
  and Japanese) now starts with an `[English] | [日本語]` switcher
  pointing at its sibling.

### Changed

- 6.2: `crates/othello-player` now depends on the workspace `tracing` crate so
  tree-reuse fallback paths can emit `tracing::warn!` / `tracing::debug!` logs
  on state mismatch instead of panicking.
- 6.4: `othello-cli` now depends on `othello-nn` and `candle-core`. Other
  crates are unaffected.
- 6.4: `othello-py::build_opponent` switches from `PlayerSpec::build_player`
  to `try_build_player` so unsupported `Nn` SPECs surface as a Python
  `ValueError` instead of panicking.

## [0.1.0-phase5] - 2026-05-09

Phase 5: external-engine integration, Python toolchain, progress bar, evaluator overlay.

### Added

- `othello-player::external` module with `ExternalEnginePlayer`, `ExternalEngineConfig`,
  `Protocol::{Gtp, Ntest}` and `EngineProtocol` trait. Synchronous IO via
  `std::process::{Command, ChildStdin, ChildStdout}` and a `mpsc::channel`-backed
  per-move timeout (no `tokio` dependency).
- `external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,arg=VAL,...]` syntax
  in `parse_player_spec`.
- Bash mock-engine scripts (`tests/mock_engine_gtp.sh`, `tests/mock_engine_ntest.sh`)
  and corresponding Unix-only integration tests in `tests/external_engine.rs`.
- `Evaluator` trait in `othello-player::traits`; `MctsPlayer` now implements it,
  exposing normalized root-visit counts after each `select_move`.
- `Player::evaluator(&mut self) -> Option<&mut dyn Evaluator>` default method
  (returns `None`); MCTS overrides to return `Some(self)`.
- `othello-engine::ProgressCallback` trait + `BatchConfig.progress` field.
  `BatchRunner` invokes the callback after each game completes.
- `--no-progress` flag and `indicatif::ProgressBar` integration in `selfplay`.
- Global `--log-file PATH` flag in `othello-cli`. Writes JSON-formatted tracing
  events to a file in addition to stderr.
- `crates/othello-tui::EvaluatorEntry` / `EvaluatorOverlay` types and an
  Evaluator overlay panel in Observe mode (top 5 legal moves with bar chart).
- `tools/visualize/` (uv workspace member): `plot-game`, `plot-curve`,
  `board_render` for matplotlib-based frame rendering and learning-curve plots.
- `tools/analyze/` (uv workspace member): `analyze-stats`, `analyze-compare`,
  `load_records`, `load_jsonl`. Pandas-based.
- `tools/tb_converter/` (uv workspace member): `jsonl-to-tb` CLI to write
  TensorBoard event files via `tensorboardX`.
- Root `pyproject.toml` registering the four Python packages (`tools/visualize`,
  `tools/analyze`, `tools/tb_converter`, `crates/othello-py`) as a uv workspace.
- `crates/othello-cli/tests/inspect_wthor.rs`: end-to-end test of the WTHOR
  inspect aggregation by invoking the built `othello-cli` binary.
- `CHANGELOG.md` (this file).

### Changed

- `BatchConfig` gained a `progress: Option<Arc<dyn ProgressCallback>>` field
  (`Default` returns `None`; existing call sites updated).
- `parse_player_spec` test for `external:` no longer expects
  `SpecError::UnsupportedKind`; replaced with positive parse / build tests.
- TUI `AppState` gained an `evaluator: Option<EvaluatorOverlay>` field.
- `crates/othello-tui/tests/snapshot.rs` Observe snapshot regenerated to
  accommodate the new layout (taller terminal, evaluator panel below
  Players).
- `README.md` rewritten to reflect Phase 5 surface area; CLAUDE.md updated.

### Notes

- External engine binaries (Edax / Egaroucid) are not exercised in CI;
  protocol-level testing is performed against bash mock scripts.
- TUI Evaluator overlay updates only after `select_move` completes (no
  intra-search updates).
- Python tools require `uv` to install; `cargo` does not depend on them.

## [0.1.0-phase4] - earlier

Phase 4: RL environments, Python bindings, batch self-play, MCTS player, TUI Observe mode.

### Added

- `crates/othello-rl/`: `OthelloEnv` (Gymnasium-compatible) and
  `OthelloMultiEnv` (PettingZoo-compatible). Observation modes: Planes (3
  channels), Flat, MoveSequence. Reward modes: Sparse, Dense.
- `crates/othello-py/`: PyO3 bindings (`maturin develop`).
- `crates/othello-engine::BatchRunner`: rayon-parallel multi-game runner with
  per-game JSON record output and aggregate JSONL log.
- `othello-cli selfplay` subcommand.
- `othello-cli observe` subcommand.
- `crates/othello-player::MctsPlayer`: UCT MCTS with configurable simulation
  count, exploration constant, seed, and rollout depth.
- `mcts:N[,c=F][,seed=M][,depth=D]` `parse_player_spec` syntax.

## [0.1.0-phase3] - earlier

Phase 3: TUI, JSONL logging, WTHOR reader, `convert` / `inspect` subcommands.

### Added

- `crates/othello-tui/`: ratatui frontend with Play and Replay modes.
- `crates/othello-io::wthor`: WTHOR (.wtb) binary record reader (read-only).
- `crates/othello-io::jsonl_log`: JSONL event logger (`game_start`, `move`,
  `pass`, `game_end`).
- `othello-cli convert` subcommand (WTHOR/JSON/GGF → JSON/GGF).
- `othello-cli inspect` subcommand (record statistics).
- `othello-cli replay` subcommand (TUI Replay mode launcher).

## [0.1.0-phase2] - earlier

Phase 2: game-loop layer.

### Added

- `crates/othello-player/`: `Player` trait and `RandomPlayer`, `GreedyPlayer`,
  `HumanPlayer` implementations. `parse_player_spec` for `random[:seed=N]` and
  `greedy`.
- `crates/othello-engine/`: `GameEngine`, full-snapshot `GameHistory`,
  `Replayer` with O(1) forward / backward / jump.
- `crates/othello-io/`: `GameRecord` neutral representation, JSON and GGF
  readers and writers.
- `crates/othello-cli/`: `play` and `simulate` subcommands.

## [0.1.0-phase1] - earlier

Phase 1: core game.

### Added

- `crates/othello-core/`: `Color`, `Coord`, `Move`, `BoardSize`, `Bitboard8`,
  `GenericBoard`, hybrid `Board` enum, `GameState`, `GameResult`,
  `OthelloError`. Hand-tuned 8×8 bitboard legal-move generation.
- `criterion` benchmarks (`legal_moves`, `self_play`).
- `proptest`-based equivalence tests between `Bitboard8` and `GenericBoard`.
