# Changelog

All notable changes to `rs-othello-sim` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
