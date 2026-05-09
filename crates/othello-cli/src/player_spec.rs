//! プレイヤー指定 SPEC のパース．設計書 §5.3 の文法のサブセット ( random / greedy のみ)．
//!
//! 例:
//! - `random` ( seed = 0)
//! - `random:seed=42`
//! - `greedy`
//!
//! Phase 4/5 で `mcts:1000`，`external:PATH` を追加する想定．

use anyhow::{Result, bail};
use othello_core::Color;
use othello_player::{GreedyPlayer, Player, RandomPlayer};

/// プレイヤー仕様 ( パース結果)．
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSpec {
    /// `random[:seed=N]`
    Random {
        /// 乱数 seed．
        seed: u64,
    },
    /// `greedy`
    Greedy,
}

/// `<KIND>[:k=v[,k=v]*]` の形式をパースする．
pub fn parse_player_spec(input: &str) -> Result<PlayerSpec> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("empty player spec");
    }
    let (kind, rest) = match trimmed.split_once(':') {
        Some((k, r)) => (k, Some(r)),
        None => (trimmed, None),
    };

    let mut params = std::collections::HashMap::new();
    if let Some(rest) = rest {
        for kv in rest.split(',') {
            let kv = kv.trim();
            if kv.is_empty() {
                continue;
            }
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("invalid param ( expected k=v): {kv:?}"))?;
            params.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    match kind.to_ascii_lowercase().as_str() {
        "random" => {
            let seed: u64 = match params.get("seed") {
                Some(s) => s
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid seed value {s:?}: {e}"))?,
                None => 0,
            };
            Ok(PlayerSpec::Random { seed })
        }
        "greedy" => Ok(PlayerSpec::Greedy),
        other => {
            bail!("unsupported player spec: {other:?} (Phase 2 supports random and greedy only)")
        }
    }
}

/// `PlayerSpec` から `Player` を `Box<dyn Player>` で生成する．
pub fn build_player(spec: &PlayerSpec, color: Color) -> Box<dyn Player> {
    match spec {
        PlayerSpec::Random { seed } => Box::new(RandomPlayer::with_seed(color, *seed)),
        PlayerSpec::Greedy => Box::new(GreedyPlayer::with_color(color)),
    }
}

/// `PlayerSpec` を JSON value に変換 ( `PlayerInfo.params` に格納する)．
pub fn spec_params(spec: &PlayerSpec) -> serde_json::Value {
    match spec {
        PlayerSpec::Random { seed } => serde_json::json!({"seed": seed}),
        PlayerSpec::Greedy => serde_json::json!({}),
    }
}

/// `PlayerSpec` の表示名．
pub fn spec_name(spec: &PlayerSpec) -> &'static str {
    match spec {
        PlayerSpec::Random { .. } => "RandomPlayer",
        PlayerSpec::Greedy => "GreedyPlayer",
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
    fn parse_random_no_seed_defaults_zero() {
        let s = parse_player_spec("random").unwrap();
        assert_eq!(s, PlayerSpec::Random { seed: 0 });
    }

    #[test]
    fn parse_greedy() {
        let s = parse_player_spec("greedy").unwrap();
        assert_eq!(s, PlayerSpec::Greedy);
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(parse_player_spec("mcts:1000").is_err());
        assert!(parse_player_spec("").is_err());
    }

    #[test]
    fn rejects_invalid_kv() {
        assert!(parse_player_spec("random:nokv").is_err());
    }
}
