//! プレイヤー指定 SPEC のパース．設計書 §5.3 の文法に対応する．
//!
//! ## 文法
//!
//! ```text
//! random[:seed=N]
//! greedy
//! mcts:N[,c=F][,seed=M][,depth=D]
//! ```
//!
//! - `mcts:N` は `simulations=N` の MCTS を指定する．
//! - `c=F` は UCT 定数 ( デフォルト $\sqrt{2}$)．
//! - `seed=M` は乱数 seed ( 省略時は OS 乱数)．
//! - `depth=D` はロールアウトの最大深さ ( デフォルト 200)．
//!
//! ## 例
//!
//! - `random` ( seed = 0)
//! - `random:seed=42`
//! - `greedy`
//! - `mcts:1000`
//! - `mcts:500,c=1.5,seed=42`

use crate::{GreedyPlayer, MctsConfig, MctsPlayer, Player, RandomPlayer};
use othello_core::Color;
use std::collections::HashMap;

/// SPEC のパースエラー．
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    /// 空文字列など，フォーマットそのものが破綻している．
    #[error("invalid player spec: {0}")]
    Format(String),
    /// パラメータの値が不正．
    #[error("invalid parameter {key}={value}: {reason}")]
    InvalidParam {
        /// パラメータキー．
        key: String,
        /// 値．
        value: String,
        /// 詳細．
        reason: String,
    },
    /// 認識できない種別．
    #[error("unsupported player kind: {0:?}")]
    UnsupportedKind(String),
}

/// プレイヤー仕様 ( パース結果)．
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerSpec {
    /// `random[:seed=N]`
    Random {
        /// 乱数 seed．
        seed: u64,
    },
    /// `greedy`
    Greedy,
    /// `mcts:N[,c=F][,seed=M][,depth=D]`
    Mcts {
        /// シミュレーション回数．
        simulations: u32,
        /// UCT 定数 $c$．
        exploration: f64,
        /// 乱数 seed ( `None` なら OS 乱数)．
        seed: Option<u64>,
        /// ロールアウト最大深さ．
        max_rollout_depth: u32,
    },
}

impl PlayerSpec {
    /// SPEC を `Box<dyn Player>` に変換する．
    #[must_use]
    pub fn build_player(&self, color: Color) -> Box<dyn Player> {
        build_player(self, color)
    }
}

/// `<KIND>[:k=v[,k=v]*]` または `<KIND>:N[,k=v]*` の形式をパースする．
///
/// `mcts:N[,...]` のように先頭が数値の場合は `simulations=N` として扱う．
pub fn parse_player_spec(input: &str) -> Result<PlayerSpec, SpecError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SpecError::Format("empty player spec".into()));
    }
    let (kind, rest) = match trimmed.split_once(':') {
        Some((k, r)) => (k, Some(r)),
        None => (trimmed, None),
    };

    let kind_lower = kind.to_ascii_lowercase();
    let mut params: HashMap<String, String> = HashMap::new();
    let mut positional: Option<String> = None;

    // `mcts` のみ先頭位置引数 ( simulations) を許可する．
    let allow_positional = kind_lower == "mcts";

    if let Some(rest) = rest {
        for (idx, kv) in rest.split(',').enumerate() {
            let kv = kv.trim();
            if kv.is_empty() {
                continue;
            }
            if let Some((k, v)) = kv.split_once('=') {
                params.insert(k.trim().to_string(), v.trim().to_string());
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

/// `PlayerSpec` から `Box<dyn Player>` を生成する．
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
    }
}

/// `PlayerSpec` の表示名 ( `name()` の値)．
#[must_use]
pub fn spec_name(spec: &PlayerSpec) -> &'static str {
    match spec {
        PlayerSpec::Random { .. } => "RandomPlayer",
        PlayerSpec::Greedy => "GreedyPlayer",
        PlayerSpec::Mcts { .. } => "MctsPlayer",
    }
}

/// `PlayerSpec` を JSON value に変換 ( `PlayerInfo.params` に格納する)．
///
/// `serde_json::Value` を返す．依存を増やさないため `serde_json::json!` ではなく手で構築する．
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
        assert!(matches!(
            parse_player_spec("external:path=/bin/edax"),
            Err(SpecError::UnsupportedKind(_))
        ));
    }

    #[test]
    fn rejects_bad_kv() {
        // `nokv` は positional 扱いが許されるのは先頭のみ
        assert!(parse_player_spec("random:nokv").is_err());
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
    }
}
