//! Parser for player SPEC strings. Matches the grammar in §5.3 of the
//! design document.
//!
//! ## Grammar
//!
//! ```text
//! random[:seed=N]
//! greedy
//! mcts:N[,c=F][,seed=M][,depth=D]
//! external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,...]
//! nn:safetensors:PATH[,temperature=F][,deterministic][,seed=N]
//! nn:onnx:PATH[,temperature=F][,deterministic][,seed=N]
//! ```
//!
//! - `mcts:N` selects MCTS with `simulations=N`.
//! - `c=F` is the UCT exploration constant (default $\sqrt{2}$).
//! - `seed=M` is the random seed (defaults to OS randomness when
//!   omitted).
//! - `depth=D` is the maximum rollout depth (default 200).
//! - `external:PATH` runs an external engine process. `protocol` is
//!   `gtp` (default) or `ntest`. Multiple `arg=VAL` are passed as
//!   command-line arguments in order. `timeout` is per-move seconds
//!   (default 30).
//! - `nn:safetensors:PATH` / `nn:onnx:PATH` selects an NN-based
//!   evaluator (Phase 6.4). Construction requires the `othello-nn`
//!   crate, so this crate only parses the SPEC; the actual
//!   `Box<dyn Player>` is built by
//!   `othello-cli::player_spec_with_nn::build_player`.
//!   - `temperature=F` is the softmax temperature (default 1.0).
//!   - `deterministic` switches to argmax selection (no value).
//!   - `seed=N` fixes the random seed for reproducibility.
//!
//! ## Examples
//!
//! - `random` (seed = 0)
//! - `random:seed=42`
//! - `greedy`
//! - `mcts:1000`
//! - `mcts:500,c=1.5,seed=42`
//! - `external:/usr/local/bin/edax,protocol=ntest,timeout=10`
//! - `external:./engines/egaroucid,protocol=gtp,arg=--level,arg=1`
//! - `nn:safetensors:./model.safetensors,deterministic`
//! - `nn:onnx:./policy.onnx,temperature=0.5,seed=7`

use crate::external::Protocol;
use crate::{
    ExternalEngineConfig, ExternalEnginePlayer, GreedyPlayer, MctsConfig, MctsPlayer, Player,
    RandomPlayer,
};
use othello_core::{BoardSize, Color};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// SPEC parse error.
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    /// The format itself is broken (e.g. empty string).
    #[error("invalid player spec: {0}")]
    Format(String),
    /// Invalid parameter value.
    #[error("invalid parameter {key}={value}: {reason}")]
    InvalidParam {
        /// Parameter key.
        key: String,
        /// Parameter value.
        value: String,
        /// Details.
        reason: String,
    },
    /// Unsupported player kind.
    #[error("unsupported player kind: {0:?}")]
    UnsupportedKind(String),
    /// Building an `nn:...` player is outside this crate's scope. The
    /// caller (e.g. `othello-cli`) is expected to build it via
    /// `othello-nn`.
    #[error("nn player requires the `othello-nn` backend; use `othello-cli` build_player")]
    NeedsNnBackend,
}

/// Backend type of the NN evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NnBackend {
    /// Candle-native safetensors file.
    Safetensors,
    /// ONNX file.
    Onnx,
}

impl NnBackend {
    /// String representation.
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Safetensors => "safetensors",
            Self::Onnx => "onnx",
        }
    }
}

/// Parse result for `nn:safetensors:PATH[,...]` / `nn:onnx:PATH[,...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct NnSpec {
    /// Backend type.
    pub backend: NnBackend,
    /// Path to the weights file.
    pub path: PathBuf,
    /// Softmax temperature (`None` defaults to 1.0).
    pub temperature: Option<f32>,
    /// `true` to switch to argmax selection.
    pub deterministic: bool,
    /// Random seed.
    pub seed: Option<u64>,
}

/// Player specification (parse result).
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerSpec {
    /// `random[:seed=N]`
    Random {
        /// Random seed.
        seed: u64,
    },
    /// `greedy`
    Greedy,
    /// `mcts:N[,c=F][,seed=M][,depth=D]`
    Mcts {
        /// Number of simulations.
        simulations: u32,
        /// UCT exploration constant $c$.
        exploration: f64,
        /// Random seed (`None` derives from OS randomness).
        seed: Option<u64>,
        /// Maximum rollout depth.
        max_rollout_depth: u32,
    },
    /// `external:PATH[,protocol=gtp|ntest][,timeout=SEC][,arg=VAL,...]`
    External {
        /// Path to the executable.
        command: PathBuf,
        /// Communication protocol.
        protocol: Protocol,
        /// Per-move timeout in seconds.
        timeout_secs: u64,
        /// Launch arguments.
        args: Vec<String>,
    },
    /// `nn:safetensors:PATH[,...]` / `nn:onnx:PATH[,...]` (Phase 6.4).
    ///
    /// The actual conversion to a `Box<dyn Player>` is handled by the
    /// `othello-cli` wrapper. This crate avoids a Candle dependency, so
    /// [`PlayerSpec::build_player`] returns
    /// [`SpecError::NeedsNnBackend`] for this variant.
    Nn(NnSpec),
}

impl PlayerSpec {
    /// Converts the SPEC into a `Box<dyn Player>` (legacy API).
    ///
    /// **Note**: panics for the `Nn` variant. To handle `Nn`, call
    /// `othello-cli::player_spec_with_nn::build_player` or use
    /// [`PlayerSpec::try_build_player`] instead.
    #[must_use]
    pub fn build_player(&self, color: Color) -> Box<dyn Player> {
        build_player(self, color)
    }

    /// Converts the SPEC into a `Box<dyn Player>` (fallible version).
    ///
    /// Returns [`SpecError::NeedsNnBackend`] for the `Nn` variant since
    /// this crate cannot construct it. The caller (which depends on
    /// `othello-nn`) handles the variant.
    pub fn try_build_player(&self, color: Color) -> Result<Box<dyn Player>, SpecError> {
        match self {
            PlayerSpec::Nn(_) => Err(SpecError::NeedsNnBackend),
            other => Ok(build_player(other, color)),
        }
    }
}

/// Parses a SPEC in the form `<KIND>[:k=v[,k=v]*]` or
/// `<KIND>:N[,k=v]*`.
///
/// When the head after `:` is numeric (e.g. `mcts:N[,...]`), it is
/// treated as `simulations=N`. When the head is a path (e.g.
/// `external:PATH[,...]`), it is treated as `command=PATH`.
///
/// `nn:safetensors:PATH[,...]` and `nn:onnx:PATH[,...]` strip the leading
/// `nn:` first and then parse the backend, path, and options.
pub fn parse_player_spec(input: &str) -> Result<PlayerSpec, SpecError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SpecError::Format("empty player spec".into()));
    }
    // `nn:...` は別経路でパースする ( 二段ヘッダ)．
    if let Some(rest) = trimmed
        .strip_prefix("nn:")
        .or_else(|| trimmed.strip_prefix("NN:"))
    {
        return parse_nn_spec(rest);
    }
    let (kind, rest) = match trimmed.split_once(':') {
        Some((k, r)) => (k, Some(r)),
        None => (trimmed, None),
    };

    let kind_lower = kind.to_ascii_lowercase();
    let mut params: HashMap<String, String> = HashMap::new();
    let mut positional: Option<String> = None;
    // `external` の `arg=VAL` のように複数許可するキーは別ベクタに集める．
    let mut multi_args: Vec<String> = Vec::new();

    // `mcts` / `external` のみ先頭位置引数を許可する．
    let allow_positional = kind_lower == "mcts" || kind_lower == "external";
    let kind_is_external = kind_lower == "external";

    if let Some(rest) = rest {
        for (idx, kv) in rest.split(',').enumerate() {
            let kv = kv.trim();
            if kv.is_empty() {
                continue;
            }
            if let Some((k, v)) = kv.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim().to_string();
                if kind_is_external && key == "arg" {
                    multi_args.push(val);
                } else {
                    params.insert(key, val);
                }
            } else if idx == 0 && allow_positional {
                positional = Some(kv.to_string());
            } else {
                return Err(SpecError::Format(format!(
                    "invalid param ( expected k=v): {kv:?}"
                )));
            }
        }
    }

    match kind_lower.as_str() {
        "random" => {
            let seed: u64 = parse_optional_u64(&params, "seed")?.unwrap_or(0);
            Ok(PlayerSpec::Random { seed })
        }
        "greedy" => Ok(PlayerSpec::Greedy),
        "mcts" => {
            let simulations: u32 = if let Some(p) = positional {
                p.parse().map_err(|e| SpecError::InvalidParam {
                    key: "simulations".into(),
                    value: p.clone(),
                    reason: format!("{e}"),
                })?
            } else if let Some(s) = params.get("simulations") {
                s.parse().map_err(|e| SpecError::InvalidParam {
                    key: "simulations".into(),
                    value: s.clone(),
                    reason: format!("{e}"),
                })?
            } else {
                return Err(SpecError::Format(
                    "mcts requires simulations count: e.g. `mcts:1000`".into(),
                ));
            };
            let exploration: f64 = match params.get("c") {
                Some(s) => s.parse().map_err(|e| SpecError::InvalidParam {
                    key: "c".into(),
                    value: s.clone(),
                    reason: format!("{e}"),
                })?,
                None => std::f64::consts::SQRT_2,
            };
            let seed = parse_optional_u64(&params, "seed")?;
            let max_rollout_depth = parse_optional_u64(&params, "depth")?
                .map(|v| v as u32)
                .unwrap_or(200);
            Ok(PlayerSpec::Mcts {
                simulations,
                exploration,
                seed,
                max_rollout_depth,
            })
        }
        "external" => {
            let command_str = if let Some(p) = positional {
                p
            } else if let Some(s) = params.get("command") {
                s.clone()
            } else {
                return Err(SpecError::Format(
                    "external requires command path: e.g. `external:./engine`".into(),
                ));
            };
            let command = PathBuf::from(command_str);
            let protocol = match params.get("protocol") {
                Some(s) => Protocol::parse(s).map_err(|e| SpecError::InvalidParam {
                    key: "protocol".into(),
                    value: s.clone(),
                    reason: e,
                })?,
                None => Protocol::Gtp,
            };
            let timeout_secs = parse_optional_u64(&params, "timeout")?.unwrap_or(30);
            Ok(PlayerSpec::External {
                command,
                protocol,
                timeout_secs,
                args: multi_args,
            })
        }
        other => Err(SpecError::UnsupportedKind(other.to_string())),
    }
}

fn parse_optional_u64(
    params: &HashMap<String, String>,
    key: &str,
) -> Result<Option<u64>, SpecError> {
    match params.get(key) {
        Some(s) => s.parse().map(Some).map_err(|e| SpecError::InvalidParam {
            key: key.into(),
            value: s.clone(),
            reason: format!("{e}"),
        }),
        None => Ok(None),
    }
}

/// Parses the remainder after stripping the `nn:` prefix
/// (e.g. `safetensors:./model.safetensors,deterministic`).
fn parse_nn_spec(rest: &str) -> Result<PlayerSpec, SpecError> {
    // 残りは `<backend>:<path>[,k=v|flag]*` の形．backend 名は `safetensors` か `onnx`．
    let (backend_str, after_backend) = rest.split_once(':').ok_or_else(|| {
        SpecError::Format("nn requires a backend: nn:safetensors:PATH or nn:onnx:PATH".into())
    })?;
    let backend = match backend_str.trim().to_ascii_lowercase().as_str() {
        "safetensors" => NnBackend::Safetensors,
        "onnx" => NnBackend::Onnx,
        other => {
            return Err(SpecError::InvalidParam {
                key: "backend".into(),
                value: other.to_string(),
                reason: "expected `safetensors` or `onnx`".into(),
            });
        }
    };

    // パスとオプションを分割．カンマで切るが先頭セグメントがパスとして扱われる．
    let mut parts = after_backend.split(',');
    let path_str = parts
        .next()
        .ok_or_else(|| SpecError::Format("nn requires a path: nn:<backend>:PATH".into()))?
        .trim();
    if path_str.is_empty() {
        return Err(SpecError::Format("nn path is empty".into()));
    }
    let path = PathBuf::from(path_str);

    let mut temperature: Option<f32> = None;
    let mut deterministic = false;
    let mut seed: Option<u64> = None;

    for kv in parts {
        let kv = kv.trim();
        if kv.is_empty() {
            continue;
        }
        if let Some((k, v)) = kv.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "temperature" => {
                    let t: f32 = val.parse().map_err(|e| SpecError::InvalidParam {
                        key: "temperature".into(),
                        value: val.into(),
                        reason: format!("{e}"),
                    })?;
                    if !(t.is_finite() && t > 0.0) {
                        return Err(SpecError::InvalidParam {
                            key: "temperature".into(),
                            value: val.into(),
                            reason: "must be finite and > 0".into(),
                        });
                    }
                    temperature = Some(t);
                }
                "seed" => {
                    let s: u64 = val.parse().map_err(|e| SpecError::InvalidParam {
                        key: "seed".into(),
                        value: val.into(),
                        reason: format!("{e}"),
                    })?;
                    seed = Some(s);
                }
                other => {
                    return Err(SpecError::Format(format!(
                        "unknown nn parameter: {other:?}"
                    )));
                }
            }
        } else if kv.eq_ignore_ascii_case("deterministic") {
            deterministic = true;
        } else {
            return Err(SpecError::Format(format!("invalid nn token: {kv:?}")));
        }
    }

    Ok(PlayerSpec::Nn(NnSpec {
        backend,
        path,
        temperature,
        deterministic,
        seed,
    }))
}

/// Builds a `Box<dyn Player>` from a `PlayerSpec`.
///
/// **Note**: panics for the `Nn` variant. To handle `Nn`, use
/// `othello-cli::player_spec_with_nn::build_player` or
/// [`PlayerSpec::try_build_player`].
#[must_use]
pub fn build_player(spec: &PlayerSpec, color: Color) -> Box<dyn Player> {
    match spec {
        PlayerSpec::Random { seed } => Box::new(RandomPlayer::with_seed(color, *seed)),
        PlayerSpec::Greedy => Box::new(GreedyPlayer::with_color(color)),
        PlayerSpec::Mcts {
            simulations,
            exploration,
            seed,
            max_rollout_depth,
        } => {
            let mut cfg = MctsConfig::new(*simulations)
                .with_exploration(*exploration)
                .with_max_rollout_depth(*max_rollout_depth);
            if let Some(s) = seed {
                cfg = cfg.with_seed(*s);
            }
            Box::new(MctsPlayer::new(color, cfg))
        }
        PlayerSpec::External {
            command,
            protocol,
            timeout_secs,
            args,
        } => {
            let cfg = ExternalEngineConfig {
                command: command.clone(),
                args: args.clone(),
                protocol: *protocol,
                board_size: BoardSize::STANDARD,
                timeout: Duration::from_secs(*timeout_secs),
                working_dir: None,
                env: Vec::new(),
            };
            Box::new(ExternalEnginePlayer::new(color, cfg))
        }
        PlayerSpec::Nn(_) => {
            panic!(
                "PlayerSpec::Nn cannot be built from othello-player; \
                 use othello-cli::player_spec_with_nn::build_player or \
                 PlayerSpec::try_build_player"
            );
        }
    }
}

/// Display name of the `PlayerSpec` (the value returned by `name()`).
#[must_use]
pub fn spec_name(spec: &PlayerSpec) -> &'static str {
    match spec {
        PlayerSpec::Random { .. } => "RandomPlayer",
        PlayerSpec::Greedy => "GreedyPlayer",
        PlayerSpec::Mcts { .. } => "MctsPlayer",
        PlayerSpec::External { .. } => "ExternalEnginePlayer",
        PlayerSpec::Nn(_) => "NnEvaluator",
    }
}

/// Converts a `PlayerSpec` into a JSON value to embed in
/// `PlayerInfo.params`.
///
/// Returns a `serde_json::Value`. Built up manually rather than via
/// `serde_json::json!` to avoid pulling in extra dependencies.
#[must_use]
pub fn spec_params(spec: &PlayerSpec) -> serde_json::Value {
    match spec {
        PlayerSpec::Random { seed } => {
            let mut m = serde_json::Map::new();
            m.insert("seed".into(), serde_json::Value::from(*seed));
            serde_json::Value::Object(m)
        }
        PlayerSpec::Greedy => serde_json::Value::Object(serde_json::Map::new()),
        PlayerSpec::Mcts {
            simulations,
            exploration,
            seed,
            max_rollout_depth,
        } => {
            let mut m = serde_json::Map::new();
            m.insert("simulations".into(), serde_json::Value::from(*simulations));
            m.insert("exploration".into(), serde_json::Value::from(*exploration));
            if let Some(s) = seed {
                m.insert("seed".into(), serde_json::Value::from(*s));
            }
            m.insert(
                "max_rollout_depth".into(),
                serde_json::Value::from(*max_rollout_depth),
            );
            serde_json::Value::Object(m)
        }
        PlayerSpec::External {
            command,
            protocol,
            timeout_secs,
            args,
        } => {
            let mut m = serde_json::Map::new();
            m.insert(
                "command".into(),
                serde_json::Value::from(command.display().to_string()),
            );
            m.insert(
                "protocol".into(),
                serde_json::Value::from(protocol.as_str()),
            );
            m.insert(
                "timeout_secs".into(),
                serde_json::Value::from(*timeout_secs),
            );
            m.insert(
                "args".into(),
                serde_json::Value::Array(
                    args.iter().cloned().map(serde_json::Value::from).collect(),
                ),
            );
            serde_json::Value::Object(m)
        }
        PlayerSpec::Nn(spec) => {
            let mut m = serde_json::Map::new();
            m.insert(
                "backend".into(),
                serde_json::Value::from(spec.backend.as_str()),
            );
            m.insert(
                "path".into(),
                serde_json::Value::from(spec.path.display().to_string()),
            );
            if let Some(t) = spec.temperature {
                m.insert("temperature".into(), serde_json::Value::from(t as f64));
            }
            m.insert(
                "deterministic".into(),
                serde_json::Value::from(spec.deterministic),
            );
            if let Some(s) = spec.seed {
                m.insert("seed".into(), serde_json::Value::from(s));
            }
            serde_json::Value::Object(m)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_random_with_seed() {
        let s = parse_player_spec("random:seed=42").unwrap();
        assert_eq!(s, PlayerSpec::Random { seed: 42 });
    }

    #[test]
    fn parse_random_default_seed() {
        let s = parse_player_spec("random").unwrap();
        assert_eq!(s, PlayerSpec::Random { seed: 0 });
    }

    #[test]
    fn parse_greedy() {
        let s = parse_player_spec("greedy").unwrap();
        assert_eq!(s, PlayerSpec::Greedy);
    }

    #[test]
    fn parse_mcts_simulations_only() {
        let s = parse_player_spec("mcts:100").unwrap();
        match s {
            PlayerSpec::Mcts {
                simulations,
                exploration,
                seed,
                max_rollout_depth,
            } => {
                assert_eq!(simulations, 100);
                assert!((exploration - std::f64::consts::SQRT_2).abs() < 1e-9);
                assert_eq!(seed, None);
                assert_eq!(max_rollout_depth, 200);
            }
            _ => panic!("expected Mcts"),
        }
    }

    #[test]
    fn parse_mcts_full() {
        let s = parse_player_spec("mcts:500,c=1.5,seed=42,depth=80").unwrap();
        match s {
            PlayerSpec::Mcts {
                simulations,
                exploration,
                seed,
                max_rollout_depth,
            } => {
                assert_eq!(simulations, 500);
                assert!((exploration - 1.5).abs() < 1e-9);
                assert_eq!(seed, Some(42));
                assert_eq!(max_rollout_depth, 80);
            }
            _ => panic!("expected Mcts"),
        }
    }

    #[test]
    fn parse_mcts_named_simulations() {
        let s = parse_player_spec("mcts:simulations=200,seed=1").unwrap();
        match s {
            PlayerSpec::Mcts {
                simulations, seed, ..
            } => {
                assert_eq!(simulations, 200);
                assert_eq!(seed, Some(1));
            }
            _ => panic!("expected Mcts"),
        }
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_player_spec("").is_err());
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(matches!(
            parse_player_spec("nnplayer"),
            Err(SpecError::UnsupportedKind(_))
        ));
    }

    #[test]
    fn parse_external_basic() {
        let s = parse_player_spec("external:/bin/edax").unwrap();
        match s {
            PlayerSpec::External {
                command,
                protocol,
                timeout_secs,
                args,
            } => {
                assert_eq!(command, PathBuf::from("/bin/edax"));
                assert_eq!(protocol, Protocol::Gtp);
                assert_eq!(timeout_secs, 30);
                assert!(args.is_empty());
            }
            _ => panic!("expected External"),
        }
    }

    #[test]
    fn parse_external_full() {
        let s = parse_player_spec(
            "external:./engines/edax,protocol=ntest,timeout=10,arg=--level,arg=1",
        )
        .unwrap();
        match s {
            PlayerSpec::External {
                command,
                protocol,
                timeout_secs,
                args,
            } => {
                assert_eq!(command, PathBuf::from("./engines/edax"));
                assert_eq!(protocol, Protocol::Ntest);
                assert_eq!(timeout_secs, 10);
                assert_eq!(args, vec!["--level".to_string(), "1".to_string()]);
            }
            _ => panic!("expected External"),
        }
    }

    #[test]
    fn parse_external_command_keyed() {
        // `command=PATH` でも指定できる
        let s = parse_player_spec("external:command=/bin/edax,protocol=gtp").unwrap();
        match s {
            PlayerSpec::External { command, .. } => {
                assert_eq!(command, PathBuf::from("/bin/edax"));
            }
            _ => panic!("expected External"),
        }
    }

    #[test]
    fn parse_external_invalid_protocol() {
        let r = parse_player_spec("external:/bin/edax,protocol=foo");
        assert!(matches!(r, Err(SpecError::InvalidParam { .. })));
    }

    #[test]
    fn rejects_bad_kv() {
        // `nokv` は positional 扱いが許されるのは先頭のみ
        assert!(parse_player_spec("random:nokv").is_err());
    }

    #[test]
    fn parse_nn_safetensors_basic() {
        let s = parse_player_spec("nn:safetensors:./model.safetensors").unwrap();
        match s {
            PlayerSpec::Nn(spec) => {
                assert_eq!(spec.backend, NnBackend::Safetensors);
                assert_eq!(spec.path, PathBuf::from("./model.safetensors"));
                assert_eq!(spec.temperature, None);
                assert!(!spec.deterministic);
                assert_eq!(spec.seed, None);
            }
            _ => panic!("expected Nn"),
        }
    }

    #[test]
    fn parse_nn_onnx_full() {
        let s = parse_player_spec("nn:onnx:/tmp/policy.onnx,temperature=0.5,deterministic,seed=7")
            .unwrap();
        match s {
            PlayerSpec::Nn(spec) => {
                assert_eq!(spec.backend, NnBackend::Onnx);
                assert_eq!(spec.path, PathBuf::from("/tmp/policy.onnx"));
                assert_eq!(spec.temperature, Some(0.5));
                assert!(spec.deterministic);
                assert_eq!(spec.seed, Some(7));
            }
            _ => panic!("expected Nn"),
        }
    }

    #[test]
    fn parse_nn_unknown_backend() {
        let r = parse_player_spec("nn:tflite:./model.bin");
        assert!(matches!(r, Err(SpecError::InvalidParam { .. })));
    }

    #[test]
    fn parse_nn_missing_path() {
        // `nn:safetensors` ( パス無し) → split_once(':') 失敗で Format エラー
        let r = parse_player_spec("nn:safetensors");
        assert!(matches!(r, Err(SpecError::Format(_))));
    }

    #[test]
    fn parse_nn_unknown_param() {
        let r = parse_player_spec("nn:safetensors:./m.safetensors,temperatuer=0.5");
        assert!(matches!(r, Err(SpecError::Format(_))));
    }

    #[test]
    fn parse_nn_negative_temperature() {
        let r = parse_player_spec("nn:safetensors:./m.safetensors,temperature=-1.0");
        assert!(matches!(r, Err(SpecError::InvalidParam { .. })));
    }

    #[test]
    fn try_build_player_nn_returns_err() {
        let s = parse_player_spec("nn:safetensors:./m.safetensors").unwrap();
        let r = s.try_build_player(Color::Black);
        assert!(matches!(r, Err(SpecError::NeedsNnBackend)));
    }

    #[test]
    fn spec_name_nn() {
        let s = parse_player_spec("nn:safetensors:./m.safetensors").unwrap();
        assert_eq!(spec_name(&s), "NnEvaluator");
    }

    #[test]
    fn spec_params_nn_includes_backend() {
        let s = parse_player_spec("nn:onnx:./m.onnx,deterministic,seed=7").unwrap();
        let v = spec_params(&s);
        assert_eq!(v["backend"], serde_json::Value::from("onnx"));
        assert_eq!(v["deterministic"], serde_json::Value::from(true));
        assert_eq!(v["seed"], serde_json::Value::from(7u64));
    }

    #[test]
    fn build_player_smoke() {
        let s = parse_player_spec("mcts:10,seed=7").unwrap();
        let p = build_player(&s, Color::Black);
        assert_eq!(p.name(), "MctsPlayer");
    }

    #[test]
    fn spec_name_works() {
        assert_eq!(spec_name(&PlayerSpec::Random { seed: 0 }), "RandomPlayer");
        assert_eq!(spec_name(&PlayerSpec::Greedy), "GreedyPlayer");
        assert_eq!(
            spec_name(&PlayerSpec::Mcts {
                simulations: 1,
                exploration: 1.0,
                seed: None,
                max_rollout_depth: 1,
            }),
            "MctsPlayer"
        );
        assert_eq!(
            spec_name(&PlayerSpec::External {
                command: PathBuf::from("/bin/echo"),
                protocol: Protocol::Gtp,
                timeout_secs: 30,
                args: vec![],
            }),
            "ExternalEnginePlayer"
        );
    }
}
