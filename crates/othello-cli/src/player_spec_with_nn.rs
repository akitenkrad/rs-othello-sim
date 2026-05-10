//! `PlayerSpec::Nn` を含む完全な `Box<dyn Player>` ビルダー．
//!
//! `othello-player` クレートは Candle に依存しないため，`Nn` バリアントの構築は
//! このモジュールに集約する．他のバリアントは `othello-player` の `build_player`
//! に委譲する．
//!
//! ## 使い方
//!
//! ```ignore
//! use othello_cli::player_spec_with_nn::build_player;
//! let spec = othello_player::parse_player_spec("nn:safetensors:./model.safetensors").unwrap();
//! let player = build_player(&spec, othello_core::Color::Black).unwrap();
//! ```

use anyhow::{Context, Result, bail};
use candle_core::Device;
use othello_core::{BoardSize, Color};
use othello_nn::{CandleModel, NnEvaluator, OnnxModel};
use othello_player::{NnBackend, NnSpec, Player, PlayerSpec};

/// `PlayerSpec` から `Box<dyn Player>` を生成する．`Nn` も対応する．
///
/// 既存の Random / Greedy / Mcts / External は [`othello_player::player_spec::build_player`]
/// にそのまま委譲する．
pub fn build_player(spec: &PlayerSpec, color: Color) -> Result<Box<dyn Player>> {
    match spec {
        PlayerSpec::Nn(nn_spec) => build_nn_player(nn_spec, color),
        other => Ok(other.build_player(color)),
    }
}

/// `NnSpec` から `Box<dyn Player>` を生成する．
fn build_nn_player(spec: &NnSpec, color: Color) -> Result<Box<dyn Player>> {
    let device = Device::Cpu;
    let board_size = BoardSize::STANDARD;
    match spec.backend {
        NnBackend::Safetensors => {
            let model = CandleModel::from_safetensors(&spec.path, board_size, device)
                .with_context(|| {
                    format!(
                        "failed to load safetensors weights from {}",
                        spec.path.display()
                    )
                })?;
            let mut ev = NnEvaluator::new(color, model)
                .with_name(format!("NnEvaluator(safetensors:{})", spec.path.display()));
            ev = apply_nn_options(ev, spec);
            Ok(Box::new(ev))
        }
        NnBackend::Onnx => {
            let model = OnnxModel::from_path(&spec.path, board_size).with_context(|| {
                format!("failed to load onnx model from {}", spec.path.display())
            })?;
            let mut ev = NnEvaluator::new(color, model)
                .with_name(format!("NnEvaluator(onnx:{})", spec.path.display()));
            ev = apply_nn_options(ev, spec);
            Ok(Box::new(ev))
        }
    }
}

/// 共通オプション ( temperature / deterministic / seed) を適用する．
fn apply_nn_options<M: othello_nn::NnModel + 'static>(
    mut ev: NnEvaluator<M>,
    spec: &NnSpec,
) -> NnEvaluator<M> {
    if let Some(t) = spec.temperature {
        ev = ev.with_temperature(t);
    }
    if spec.deterministic {
        ev = ev.deterministic();
    }
    if let Some(s) = spec.seed {
        ev = ev.with_seed(s);
    }
    ev
}

/// SPEC + per-game seed override から Player を生成する ( BatchRunner factory 用)．
///
/// `Random` / `Mcts` / `Nn` は `seed ^ overlay_seed` で乱数を撹拌する．
/// `External` / `Greedy` は overlay seed の影響を受けない．
pub fn build_with_seed_override(
    spec: &PlayerSpec,
    color: Color,
    overlay_seed: u64,
) -> Result<Box<dyn Player>> {
    let overridden = match spec.clone() {
        PlayerSpec::Random { seed: s } => PlayerSpec::Random {
            seed: s ^ overlay_seed,
        },
        PlayerSpec::Greedy => PlayerSpec::Greedy,
        PlayerSpec::Mcts {
            simulations,
            exploration,
            seed: s,
            max_rollout_depth,
        } => PlayerSpec::Mcts {
            simulations,
            exploration,
            seed: Some(s.unwrap_or(0) ^ overlay_seed),
            max_rollout_depth,
        },
        external @ PlayerSpec::External { .. } => external,
        PlayerSpec::Nn(mut nn_spec) => {
            let s = nn_spec.seed.unwrap_or(0) ^ overlay_seed;
            nn_spec.seed = Some(s);
            PlayerSpec::Nn(nn_spec)
        }
    };
    if let Ok(p) = build_player(&overridden, color) {
        Ok(p)
    } else {
        bail!(
            "failed to build player from spec; try with `othello-cli` build_player wrapper or check the SPEC string"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_nn_safetensors_missing_file_returns_err() {
        let spec = othello_player::parse_player_spec(
            "nn:safetensors:/tmp/__never_exists_othello.safetensors",
        )
        .unwrap();
        let r = build_player(&spec, Color::Black);
        assert!(r.is_err());
    }

    #[test]
    fn build_random_still_works() {
        let spec = othello_player::parse_player_spec("random:seed=42").unwrap();
        let p = build_player(&spec, Color::Black).unwrap();
        assert_eq!(p.name(), "RandomPlayer");
    }

    #[test]
    fn build_with_seed_override_random_xors() {
        let spec = othello_player::parse_player_spec("random:seed=42").unwrap();
        let _ = build_with_seed_override(&spec, Color::Black, 7).unwrap();
        // smoke only: 構築できれば良い
    }

    #[test]
    fn build_with_seed_override_nn_missing_file_propagates() {
        let spec =
            othello_player::parse_player_spec("nn:safetensors:/tmp/__nope__.safetensors").unwrap();
        let r = build_with_seed_override(&spec, Color::Black, 1);
        assert!(r.is_err());
    }
}
