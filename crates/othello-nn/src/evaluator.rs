//! [`NnEvaluator`]: adapter that drives an NN model as both a Player
//! and an Evaluator.
//!
//! - `select_move` chooses one move and stashes the policy internally.
//! - `evaluate` ([`othello_player::Evaluator`]) returns the most recent
//!   policy.
//!
//! Move selection steps:
//! 1. Convert `state` into a `(1, 3, H, W)` tensor.
//! 2. Call `model.forward` to obtain policy logits and value.
//! 3. softmax -> mask out illegal moves -> renormalize.
//! 4. argmax when `deterministic`, otherwise sample with temperature.
//! 5. Store the most recent policy (per legal move -> probability) in
//!    `last_policy`.

use crate::error::NnError;
use crate::input::state_to_tensor;
use crate::model::NnModel;
use candle_nn::ops::softmax;
use othello_core::{BoardSize, Color, Coord, GameResult, GameState, Move};
use othello_player::{Evaluator, Player, PlayerError};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Player that uses NN policy/value output as a move-selection
/// strategy.
///
/// Unlike standard AlphaZero-style players, no MCTS is run here — the
/// policy is **sampled directly**. MCTS integration (e.g. using the NN
/// as a prior in `MctsPlayer`) is planned for Phase 7 and beyond.
pub struct NnEvaluator<M: NnModel> {
    model: M,
    color: Color,
    name: String,
    temperature: f32,
    deterministic: bool,
    rng: ChaCha8Rng,
    seed: Option<u64>,
    last_policy: Option<HashMap<Move, f32>>,
}

impl<M: NnModel> NnEvaluator<M> {
    /// Constructs an evaluator with defaults: `temperature = 1.0`,
    /// `deterministic = false`, `seed` from OS randomness.
    pub fn new(color: Color, model: M) -> Self {
        Self {
            model,
            color,
            name: "NnEvaluator".to_string(),
            temperature: 1.0,
            deterministic: false,
            rng: ChaCha8Rng::from_entropy(),
            seed: None,
            last_policy: None,
        }
    }

    /// Builder: overrides the temperature. Requires `t > 0`. Lower
    /// values sharpen the policy peak.
    #[must_use]
    pub fn with_temperature(mut self, t: f32) -> Self {
        self.temperature = t.max(1e-6);
        self
    }

    /// Switches to argmax selection (fully deterministic).
    #[must_use]
    pub fn deterministic(mut self) -> Self {
        self.deterministic = true;
        self
    }

    /// Builder: sets the random seed for reproducibility.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self.seed = Some(seed);
        self
    }

    /// Builder: overrides the display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl<M: NnModel> NnEvaluator<M> {
    /// Internal helper: forward pass + legal-move masking + probability
    /// distribution construction.
    fn compute_policy(&self, state: &GameState) -> Result<HashMap<Move, f32>, NnError> {
        let device = self.model.device().clone();
        let input = state_to_tensor(state, self.color, &device)?;
        let (logits, _value) = self.model.forward(&input)?;
        let board_size = self.model.board_size();
        let n = (board_size.rows as usize) * (board_size.cols as usize);
        let logits_dims = logits.shape().dims().to_vec();
        if logits_dims.len() != 2 || logits_dims[0] != 1 || logits_dims[1] != n + 1 {
            return Err(NnError::OutputShape(format!(
                "expected policy logits (1, {}), got {:?}",
                n + 1,
                logits_dims
            )));
        }
        // 温度付き softmax
        let temp = self.temperature.max(1e-6);
        let scaled = (&logits * (1.0 / temp as f64))?;
        let probs = softmax(&scaled, 1)?;
        let probs_vec: Vec<f32> = probs.flatten_all()?.to_vec1::<f32>()?;
        debug_assert_eq!(probs_vec.len(), n + 1);

        // 合法手を取得．Pass しか取れない場合も legal_moves に含まれる．
        let legal = state.legal_moves();
        let mut masked = HashMap::with_capacity(legal.len());
        let mut total: f32 = 0.0;
        for mv in &legal {
            let idx = move_to_index(*mv, board_size);
            let p = probs_vec.get(idx).copied().unwrap_or(0.0);
            // モデル出力が NaN や負値の場合は 0 で安全側に倒す
            let p = if p.is_finite() && p >= 0.0 { p } else { 0.0 };
            masked.insert(*mv, p);
            total += p;
        }
        if total > 0.0 {
            for v in masked.values_mut() {
                *v /= total;
            }
        } else {
            // 全 0 ( マスク後) なら一様分布で初期化．
            let n_legal = masked.len().max(1) as f32;
            for v in masked.values_mut() {
                *v = 1.0 / n_legal;
            }
        }
        Ok(masked)
    }
}

fn move_to_index(mv: Move, size: BoardSize) -> usize {
    match mv {
        Move::Place(c) => (c.row as usize) * (size.cols as usize) + (c.col as usize),
        Move::Pass => (size.rows as usize) * (size.cols as usize),
    }
}

#[allow(dead_code)]
fn index_to_move(idx: usize, size: BoardSize) -> Move {
    let cells = (size.rows as usize) * (size.cols as usize);
    if idx >= cells {
        Move::Pass
    } else {
        let row = (idx / size.cols as usize) as u8;
        let col = (idx % size.cols as usize) as u8;
        Move::Place(Coord::new(row, col))
    }
}

impl<M: NnModel + 'static> Player for NnEvaluator<M> {
    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color {
        self.color
    }

    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError> {
        let policy = self
            .compute_policy(state)
            .map_err(|e| PlayerError::Other(format!("nn forward: {e}")))?;
        if policy.is_empty() {
            return Err(PlayerError::IllegalMove);
        }
        let chosen = if self.deterministic {
            // argmax．同点があれば最初の合法手の順序に従う ( 安定化のため
            // ソートしてから線形に最大を取る)．
            let mut best: Option<(Move, f32)> = None;
            let mut entries: Vec<(Move, f32)> = policy.iter().map(|(m, p)| (*m, *p)).collect();
            entries.sort_by(|a, b| compare_move(&a.0, &b.0));
            for (mv, p) in entries {
                match best {
                    None => best = Some((mv, p)),
                    Some((_, bp)) if p > bp => best = Some((mv, p)),
                    _ => {}
                }
            }
            best.map(|(m, _)| m).ok_or(PlayerError::IllegalMove)?
        } else {
            // 温度付きサンプリング ( compute_policy 内で温度は既に反映済)．
            let mut entries: Vec<(Move, f32)> = policy.iter().map(|(m, p)| (*m, *p)).collect();
            entries.sort_by(|a, b| compare_move(&a.0, &b.0));
            let total: f32 = entries.iter().map(|(_, p)| *p).sum();
            if total <= 0.0 {
                // フォールバック: 一様
                entries
                    .choose(&mut self.rng)
                    .map(|(m, _)| *m)
                    .ok_or(PlayerError::IllegalMove)?
            } else {
                let weights: Vec<f32> = entries.iter().map(|(_, p)| p / total).collect();
                sample_weighted(&entries, &weights, &mut self.rng)
                    .ok_or(PlayerError::IllegalMove)?
            }
        };
        self.last_policy = Some(policy);
        Ok(chosen)
    }

    fn on_game_end(&mut self, _final_state: &GameState, _result: GameResult) {}

    fn reset(&mut self) {
        self.last_policy = None;
        if let Some(seed) = self.seed {
            self.rng = ChaCha8Rng::seed_from_u64(seed);
        }
    }

    fn evaluator(&mut self) -> Option<&mut dyn Evaluator> {
        Some(self)
    }
}

impl<M: NnModel + 'static> Evaluator for NnEvaluator<M> {
    fn evaluate(&mut self, _state: &GameState) -> Option<HashMap<Move, f32>> {
        self.last_policy.clone()
    }
}

fn compare_move(a: &Move, b: &Move) -> std::cmp::Ordering {
    fn key(m: &Move) -> (u8, u8, u8) {
        match m {
            Move::Place(c) => (0, c.row, c.col),
            Move::Pass => (1, 0, 0),
        }
    }
    key(a).cmp(&key(b))
}

fn sample_weighted<R: rand::Rng + ?Sized>(
    entries: &[(Move, f32)],
    weights: &[f32],
    rng: &mut R,
) -> Option<Move> {
    if entries.is_empty() {
        return None;
    }
    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return entries.first().map(|(m, _)| *m);
    }
    let r: f32 = rng.r#gen::<f32>() * total;
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        acc += *w;
        if r <= acc {
            return Some(entries[i].0);
        }
    }
    entries.last().map(|(m, _)| *m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candle_model::CandleModel;
    use candle_core::Device;
    use othello_core::{Color, GameState};

    #[test]
    fn deterministic_mode_picks_legal_move() {
        let m = CandleModel::random_init(BoardSize::STANDARD, 0, Device::Cpu).unwrap();
        let mut p = NnEvaluator::new(Color::Black, m)
            .deterministic()
            .with_seed(1);
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        let legal = s.legal_moves();
        assert!(legal.contains(&mv), "chose illegal move {:?}", mv);
    }

    #[test]
    fn evaluate_returns_after_select() {
        let m = CandleModel::random_init(BoardSize::STANDARD, 0, Device::Cpu).unwrap();
        let mut p = NnEvaluator::new(Color::Black, m).with_seed(2);
        let s = GameState::standard_8x8();
        assert!(p.evaluate(&s).is_none());
        let _ = p.select_move(&s).unwrap();
        let e = p.evaluate(&s).expect("evaluator should populate");
        // 4 legal moves at start
        assert_eq!(e.len(), 4);
        let total: f32 = e.values().sum();
        assert!((total - 1.0).abs() < 1e-4, "total={total}");
    }

    #[test]
    fn move_index_roundtrip_8x8() {
        for r in 0..8u8 {
            for c in 0..8u8 {
                let mv = Move::Place(Coord::new(r, c));
                let i = move_to_index(mv, BoardSize::STANDARD);
                assert_eq!(index_to_move(i, BoardSize::STANDARD), mv);
            }
        }
        let i = move_to_index(Move::Pass, BoardSize::STANDARD);
        assert_eq!(i, 64);
        assert_eq!(index_to_move(64, BoardSize::STANDARD), Move::Pass);
    }
}
