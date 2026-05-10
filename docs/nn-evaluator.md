[English](nn-evaluator.md) | [日本語](ja/nn-evaluator.md)

# NN Evaluator (Phase 6.4)

`rs-othello-sim` ships a neural-network policy/value evaluator under
`crates/othello-nn/`.  It is the foundation for AlphaZero-style self-play
experiments: a model exposes a policy head over the action space and a value
head estimating winrate, and the `NnEvaluator` glue type plugs the model into
the existing `Player` and `Evaluator` traits so it can drive games and feed
the TUI overlay.

This document covers the intent, IO schema, supported formats, CLI integration
and current limitations.

## Why a separate crate

Candle pulls in a non-trivial dependency tree (gemm, half, candle-core /
candle-nn / candle-onnx).  Rather than adding it to `othello-player`, which
every other crate depends on, the NN backend lives in its own
`othello-nn` crate.  `othello-player` keeps a thin `PlayerSpec::Nn` variant so
parsing the `nn:` SPEC strings does not require Candle, and the actual
construction is delegated to `othello-cli`.

This split avoids a circular dependency between `othello-player` and
`othello-nn`, since `othello-nn` itself depends on the `Player` and
`Evaluator` traits.

## IO schema

The model is expected to operate on the same 3-channel observation that
`othello-rl` produces:

- Input:  `(batch, 3, H, W)` `f32`
  - channel 0: own stones (1.0 where the *side-to-move* has a stone)
  - channel 1: opponent stones
  - channel 2: legal-move mask (set on cells where the side-to-move can play)
- Output:
  - `policy`: `(batch, H * W + 1)` policy *logits* (softmax is applied by the
    evaluator, not the model). Index `H*W` represents `Move::Pass`.
  - `value`:  `(batch,)` (or `(batch, 1)`) tanh-space win estimate in
    `[-1, 1]` from the *side-to-move*'s perspective.

`NnEvaluator` masks out illegal moves before sampling, renormalises the
remaining probabilities, and supports either argmax (`deterministic`) or
temperature-weighted sampling.

## Supported weight formats

Two backends are provided:

| Backend       | Loader                                     | Notes |
|---------------|--------------------------------------------|-------|
| safetensors   | `CandleModel::from_safetensors(path, ...)` | Native Candle loader. Requires the model to use the architecture and parameter names defined in `candle_model.rs`. |
| ONNX          | `OnnxModel::from_path(path, ...)`          | Uses `candle_onnx::simple_eval`. Default tensor names: input `input`, outputs `policy` and `value`. Override with `OnnxModel::with_io_names`. |

Phase 6.4 ships *no pre-trained weights*.  For testing without a saved model,
`CandleModel::random_init(BoardSize, seed, Device)` builds the same
architecture with Candle's default random initialiser; this is what the
integration tests use.

The bundled architecture is a small AlphaZero-style ResNet:

```
input (B, 3, H, W)
  → conv 3x3 (3 → 32) → BN → ReLU             (stem)
  → 2 × ResidualBlock (32, 32)
  ├─ policy head: conv 1x1 (32 → 2) → BN → ReLU → Linear(2*H*W → H*W + 1)
  └─ value  head: conv 1x1 (32 → 1) → BN → ReLU → Linear(H*W → 64)
                                              → ReLU → Linear(64 → 1) → tanh
```

## Relation to the `Evaluator` trait

`NnEvaluator` implements both `othello_player::Player` and
`othello_player::Evaluator`. After every `select_move` call the per-move
probability map (`HashMap<Move, f32>`) is cached in `last_policy`; the TUI
Observe overlay reads it via `Player::evaluator()` exactly the same way it
already reads MCTS visit counts.

The trait return is `0..=1` "higher is better" semantics, so the value head is
*not* exposed through `Evaluator::evaluate` — only the (post-mask, normalised)
policy is.

## CLI usage

PlayerSpec adds two new variants:

```
nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
```

Examples:

```bash
# Argmax over the policy head, deterministic.
othello-cli simulate \
  --black "nn:safetensors:./model.safetensors,deterministic" \
  --white "random:seed=42"

# Sampled play (temperature=0.5) versus a 200-rollout MCTS.
othello-cli selfplay --num-games 100 \
  --black "nn:onnx:./policy.onnx,temperature=0.5,seed=7" \
  --white "mcts:200"

# TUI observe with policy overlay.
othello-cli observe \
  --black "nn:safetensors:./model.safetensors" \
  --white "greedy" \
  --auto-delay 500
```

Construction is performed in `othello-cli::player_spec_with_nn::build_player`,
which is the only place that links Candle. Other crates remain Candle-free.

## Current limitations

- No pre-trained weights are shipped. Random initialisation is only useful for
  smoke tests; expect uniform-ish play.
- The model architecture is fixed in code (channels = 32, 2 residual blocks).
  A more flexible config will follow once the training pipeline lands.
- ONNX integration tests are not enabled by default — they require a fixture
  file that adheres to the IO schema. The unit tests cover the loader-error
  path (missing file) only.
- AlphaZero-style MCTS that uses the policy head as a prior is *not yet
  wired*; for now the evaluator samples directly from the policy. A future
  Phase 7 task will combine `NnEvaluator` with `MctsPlayer`.
- CPU only. Metal/CUDA backends are reachable through Candle but are gated
  behind features that are not enabled in this workspace.

## Roadmap

The downstream tasks that build on Phase 6.4 are:

- Phase 6.7 Replay buffer: collect self-play trajectories produced by
  `NnEvaluator` and the existing `selfplay` runner.
- Phase 7 (planned) Training pipeline: PyTorch-side trainer that reads the
  buffer, fits the same architecture, and exports either safetensors (Candle
  native) or ONNX (interoperable) weights for `othello-cli` to load.
