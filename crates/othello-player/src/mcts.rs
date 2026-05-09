//! [`MctsPlayer`]: UCT (Upper Confidence Bound applied to Trees) ベースの MCTS プレイヤー．
//!
//! 標準 UCT による最小実装．木は手選択ごとに新規生成し，木の永続化は行わない．
//!
//! ## アルゴリズム
//!
//! 1. **Selection**: 葉ノードまで `argmax(Q/N + c * sqrt(ln(N_parent) / N))` で降下
//! 2. **Expansion**: 未展開の合法手を 1 つ展開
//! 3. **Simulation (rollout)**: ランダム合法手で終局までプレイ ( `max_rollout_depth` まで)
//! 4. **Backpropagation**: 終局結果 ( win/loss/draw → +1/-1/0) を逆伝播
//!
//! ## 例
//!
//! ```
//! use othello_core::prelude::*;
//! use othello_player::{MctsConfig, MctsPlayer, Player};
//!
//! let mut p = MctsPlayer::new(Color::Black, MctsConfig::new(50)).with_seed(42);
//! let s = GameState::standard_8x8();
//! let mv = p.select_move(&s).unwrap();
//! assert!(matches!(mv, Move::Place(_)));
//! ```

use crate::traits::{Evaluator, Player, PlayerError};
use othello_core::{Color, GameState, Move};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// MCTS プレイヤーの設定．
#[derive(Debug, Clone, Copy)]
pub struct MctsConfig {
    /// 1 手あたりのシミュレーション数．
    pub simulations: u32,
    /// UCT 定数 $c$．通常 $\sqrt{2} \approx 1.414$．
    pub exploration: f64,
    /// 乱数 seed ( `None` なら OS 乱数からの派生)．
    pub seed: Option<u64>,
    /// ロールアウトの最大深さ ( 安全装置)．
    pub max_rollout_depth: u32,
}

impl MctsConfig {
    /// `simulations` のみ指定して，他はデフォルト値を採用する．
    ///
    /// - `exploration = sqrt(2)`
    /// - `seed = None`
    /// - `max_rollout_depth = 200`
    #[must_use]
    pub fn new(simulations: u32) -> Self {
        Self {
            simulations,
            exploration: std::f64::consts::SQRT_2,
            seed: None,
            max_rollout_depth: 200,
        }
    }

    /// `seed` を設定する ( builder)．
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// `exploration` を設定する ( builder)．
    #[must_use]
    pub fn with_exploration(mut self, c: f64) -> Self {
        self.exploration = c;
        self
    }

    /// `max_rollout_depth` を設定する ( builder)．
    #[must_use]
    pub fn with_max_rollout_depth(mut self, depth: u32) -> Self {
        self.max_rollout_depth = depth;
        self
    }
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// UCT に基づく MCTS プレイヤー．
///
/// 各手選択ごとに新しい木を構築する ( 木の持ち回しは Phase 4 では行わない)．
pub struct MctsPlayer {
    name: String,
    color: Color,
    config: MctsConfig,
    rng: ChaCha8Rng,
    /// 直近 [`select_move`] / [`Evaluator::evaluate`] で得た「ルート直下の手 → ( visit, q)」．
    ///
    /// `Evaluator` 実装で正規化値を返すために保持する．
    /// `select_move` 中の中間状態は反映しない ( 完了時にのみ更新)．
    last_root_stats: Vec<(Move, u32, f64)>,
}

impl MctsPlayer {
    /// `color` と `config` を指定して生成する．
    #[must_use]
    pub fn new(color: Color, config: MctsConfig) -> Self {
        let seed = config.seed.unwrap_or_else(default_seed);
        Self {
            name: "MctsPlayer".to_string(),
            color,
            config,
            rng: ChaCha8Rng::seed_from_u64(seed),
            last_root_stats: Vec::new(),
        }
    }

    /// seed を後付けで設定する ( builder)．`MctsConfig::seed` を上書きする．
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self
    }

    /// 名前を変更する ( builder)．
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 設定の参照．
    #[inline]
    #[must_use]
    pub fn config(&self) -> &MctsConfig {
        &self.config
    }
}

impl Player for MctsPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color {
        self.color
    }

    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError> {
        let legal = state.legal_moves();
        if legal.is_empty() {
            self.last_root_stats.clear();
            return Ok(Move::Pass);
        }
        if legal.len() == 1 {
            self.last_root_stats = vec![(legal[0], 1, 0.0)];
            return Ok(legal[0]);
        }

        // 新規ツリーを構築して UCT 検索する．
        let mut tree = MctsTree::new(state.clone(), self.config.exploration);
        for _ in 0..self.config.simulations {
            tree.run_one(&mut self.rng, self.config.max_rollout_depth);
        }
        // `select_move` 完了時にルート統計を更新する ( 中間状態は反映しない)．
        self.last_root_stats = tree.root_stats();
        let best = tree.best_move().ok_or_else(|| {
            PlayerError::Other("MctsPlayer: tree returned no move ( unexpected)".into())
        })?;
        Ok(best)
    }

    fn reset(&mut self) {
        if let Some(seed) = self.config.seed {
            self.rng = ChaCha8Rng::seed_from_u64(seed);
        }
        self.last_root_stats.clear();
    }

    fn evaluator(&mut self) -> Option<&mut dyn Evaluator> {
        Some(self)
    }
}

impl Evaluator for MctsPlayer {
    /// 直近の `select_move` 呼び出しで得たルート直下の visit 数を正規化して返す．
    ///
    /// 引数 `state` が直近の root と異なる場合 ( 履歴が古い)，現状の cache をそのまま返す
    /// 実装としている ( 必要なら呼び出し側で `select_move` を先に呼ぶこと)．
    /// visit 0 の場合や cache 未生成の場合は `None` を返す．
    fn evaluate(&mut self, _state: &GameState) -> Option<HashMap<Move, f32>> {
        if self.last_root_stats.is_empty() {
            return None;
        }
        let total: u32 = self.last_root_stats.iter().map(|(_, v, _)| *v).sum();
        if total == 0 {
            return None;
        }
        let mut out = HashMap::new();
        for &(mv, visits, _q) in &self.last_root_stats {
            out.insert(mv, visits as f32 / total as f32);
        }
        Some(out)
    }
}

/// ノードに紐づく統計と未展開手のキュー．
#[derive(Debug)]
struct MctsNode {
    /// このノードに対応する局面．
    state: GameState,
    /// 親ノード ( `None` ならルート)．
    parent: Option<usize>,
    /// 親からこのノードに至る手．デバッグ・将来拡張用に保持．
    #[allow(dead_code)]
    incoming_move: Option<Move>,
    /// このノードの手番から見た累積スコア ( win = +1, draw = 0, loss = -1)．
    /// より厳密には：ロールアウト終局時の `agent_color` 視点の結果ではなく，
    /// **ノードの「これから指す側」視点** で逆算した値を保持する ( UCT の通例)．
    score_sum: f64,
    /// 訪問回数．
    visits: u32,
    /// まだ展開していない合法手．
    untried_moves: Vec<Move>,
    /// 子ノード `( move, node_id)`．
    children: Vec<(Move, usize)>,
    /// 終端ノードかどうか ( 終局済み)．
    terminal: bool,
}

impl MctsNode {
    fn new(state: GameState, parent: Option<usize>, incoming_move: Option<Move>) -> Self {
        let terminal = state.is_terminal();
        let untried_moves = if terminal {
            Vec::new()
        } else {
            let mut legal = state.legal_moves();
            if legal.is_empty() {
                // パスしか取れない局面: untried に Pass を 1 つ入れる．
                legal.push(Move::Pass);
            }
            legal
        };
        Self {
            state,
            parent,
            incoming_move,
            score_sum: 0.0,
            visits: 0,
            untried_moves,
            children: Vec::new(),
            terminal,
        }
    }

    fn fully_expanded(&self) -> bool {
        self.untried_moves.is_empty()
    }
}

struct MctsTree {
    nodes: Vec<MctsNode>,
    exploration: f64,
}

impl MctsTree {
    fn new(root_state: GameState, exploration: f64) -> Self {
        let root = MctsNode::new(root_state, None, None);
        Self {
            nodes: vec![root],
            exploration,
        }
    }

    fn run_one(&mut self, rng: &mut ChaCha8Rng, max_rollout_depth: u32) {
        // 1. Selection
        let leaf = self.select(0);
        // 2. Expansion ( 終端でなければ 1 手展開)
        let rollout_node = if !self.nodes[leaf].terminal && !self.nodes[leaf].fully_expanded() {
            self.expand(leaf, rng)
        } else {
            leaf
        };
        // 3. Simulation
        let result_for_root_player = self.rollout(rollout_node, rng, max_rollout_depth);
        // 4. Backpropagation
        self.backpropagate(rollout_node, result_for_root_player);
    }

    /// 葉まで UCT で降下し，葉ノードの id を返す．
    fn select(&self, mut node: usize) -> usize {
        loop {
            let n = &self.nodes[node];
            if n.terminal {
                return node;
            }
            if !n.fully_expanded() {
                return node;
            }
            // 完全展開済み．子の中で UCT 最大を選ぶ．
            let parent_visits = n.visits.max(1) as f64;
            let log_parent = parent_visits.ln();
            let mut best: Option<(usize, f64)> = None;
            for &(_mv, child_id) in &n.children {
                let c = &self.nodes[child_id];
                let visits = c.visits.max(1) as f64;
                let q = if c.visits == 0 {
                    0.0
                } else {
                    c.score_sum / visits
                };
                let uct = q + self.exploration * (log_parent / visits).sqrt();
                match best {
                    None => best = Some((child_id, uct)),
                    Some((_, prev)) if uct > prev => best = Some((child_id, uct)),
                    _ => {}
                }
            }
            match best {
                Some((next, _)) => node = next,
                None => return node,
            }
        }
    }

    /// 葉ノードに 1 手だけ子を追加し，新しい子ノード id を返す．
    fn expand(&mut self, leaf: usize, rng: &mut ChaCha8Rng) -> usize {
        // ランダムに 1 つの未試行手を取り出す．
        let mv = {
            let n = &mut self.nodes[leaf];
            let idx = rng.gen_range(0..n.untried_moves.len());
            n.untried_moves.swap_remove(idx)
        };
        let mut next_state = self.nodes[leaf].state.clone();
        // apply_move は不正手なら Err．`legal_moves()` 由来なので通常成功するが，
        // Pass は合法手があるときに使うとエラーになる．leaf の untried_moves 構築時に
        // 「合法手が空のときのみ Pass を入れている」ため，問題は出ないはず．
        // それでも安全のため，Err になったらそのままスキップ ( leaf 自体を返す)．
        if next_state.apply_move(mv).is_err() {
            return leaf;
        }
        let new_id = self.nodes.len();
        let child = MctsNode::new(next_state, Some(leaf), Some(mv));
        self.nodes.push(child);
        self.nodes[leaf].children.push((mv, new_id));
        new_id
    }

    /// ロールアウト．戻り値はルートの「これから指す側」視点の結果 ( +1 / 0 / -1)．
    fn rollout(&self, node_id: usize, rng: &mut ChaCha8Rng, max_depth: u32) -> f64 {
        let root_player = self.nodes[0].state.side_to_move;
        let mut state = self.nodes[node_id].state.clone();
        let mut depth = 0u32;
        while !state.is_terminal() && depth < max_depth {
            let legal = state.legal_moves();
            let mv = if legal.is_empty() {
                Move::Pass
            } else {
                *legal.choose(rng).expect("non-empty")
            };
            if state.apply_move(mv).is_err() {
                break;
            }
            depth += 1;
        }
        score_for(&state, root_player)
    }

    /// 結果 `result_for_root` ( ルートの「これから指す側」視点 +1/0/-1) を逆伝播する．
    ///
    /// 各ノードには「親で指した側 ( = 親の `side_to_move`) の視点」のスコアを蓄積する．
    /// これにより，子の選択時には「親の視点での win rate」を最大化すればよい．
    fn backpropagate(&mut self, node_id: usize, result_for_root: f64) {
        let root_side = self.nodes[0].state.side_to_move;
        let mut cur = Some(node_id);
        while let Some(id) = cur {
            // 当該ノードを生成した「指し手の主」=「親の side_to_move」の視点で記録する．
            // ルートは親が無いので例外．ルートには `root_side` 視点で記録する．
            let view = match self.nodes[id].parent {
                Some(parent_id) => self.nodes[parent_id].state.side_to_move,
                None => root_side,
            };
            let value = if view == root_side {
                result_for_root
            } else {
                -result_for_root
            };
            self.nodes[id].score_sum += value;
            self.nodes[id].visits += 1;
            cur = self.nodes[id].parent;
        }
    }

    /// ルート直下の各子の `( move, visits, q)` を返す．`q` は `score_sum / visits`．
    fn root_stats(&self) -> Vec<(Move, u32, f64)> {
        let root = &self.nodes[0];
        let mut out = Vec::with_capacity(root.children.len());
        for &(mv, child_id) in &root.children {
            let c = &self.nodes[child_id];
            let q = if c.visits == 0 {
                0.0
            } else {
                c.score_sum / c.visits as f64
            };
            out.push((mv, c.visits, q));
        }
        out
    }

    /// ルート直下の子から訪問数最大の手を選ぶ．
    fn best_move(&self) -> Option<Move> {
        let root = &self.nodes[0];
        let mut best: Option<(Move, u32)> = None;
        for &(mv, child_id) in &root.children {
            let visits = self.nodes[child_id].visits;
            match best {
                None => best = Some((mv, visits)),
                Some((_, prev)) if visits > prev => best = Some((mv, visits)),
                _ => {}
            }
        }
        best.map(|(m, _)| m)
    }
}

/// 終局 / 打ち切り状態における `view` 視点のスコア ( +1 win / 0 draw / -1 loss)．
fn score_for(state: &GameState, view: Color) -> f64 {
    let b = state.board.count(Color::Black);
    let w = state.board.count(Color::White);
    let (own, opp) = match view {
        Color::Black => (b, w),
        Color::White => (w, b),
    };
    if own > opp {
        1.0
    } else if own < opp {
        -1.0
    } else {
        0.0
    }
}

fn default_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xDEAD_BEEF_CAFE_BABE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RandomPlayer;
    use othello_core::{BoardSize, GameState};

    #[test]
    fn selects_legal_move() {
        let mut p = MctsPlayer::new(Color::Black, MctsConfig::new(50)).with_seed(7);
        let s = GameState::standard_8x8();
        let mv = p.select_move(&s).unwrap();
        assert!(s.legal_moves().contains(&mv));
    }

    #[test]
    fn deterministic_with_same_seed() {
        // 同 seed 同 config なら必ず同じ手列を返す
        let mut p1 = MctsPlayer::new(Color::Black, MctsConfig::new(80)).with_seed(123);
        let mut p2 = MctsPlayer::new(Color::Black, MctsConfig::new(80)).with_seed(123);
        let s = GameState::standard_8x8();
        for _ in 0..3 {
            assert_eq!(p1.select_move(&s).unwrap(), p2.select_move(&s).unwrap());
        }
    }

    #[test]
    fn returns_pass_when_only_pass_available() {
        // 全マス白で塞ぐ → 黒も白も置けない → 合法手なし
        let mut s = GameState::standard_8x8();
        for r in 0..8u8 {
            for c in 0..8u8 {
                s.board
                    .set(othello_core::Coord::new(r, c), Some(Color::White));
            }
        }
        let mut p = MctsPlayer::new(Color::Black, MctsConfig::new(10)).with_seed(1);
        let mv = p.select_move(&s).unwrap();
        assert_eq!(mv, Move::Pass);
    }

    #[test]
    fn works_on_4x4_board() {
        let s = GameState::standard(BoardSize::square(4)).unwrap();
        let mut p = MctsPlayer::new(s.side_to_move, MctsConfig::new(20)).with_seed(42);
        let mv = p.select_move(&s).unwrap();
        assert!(s.legal_moves().contains(&mv));
    }

    #[test]
    fn evaluator_returns_normalized_visits_after_select_move() {
        use crate::Evaluator;
        let mut p = MctsPlayer::new(Color::Black, MctsConfig::new(50)).with_seed(7);
        let s = GameState::standard_8x8();
        // select_move 前は None
        assert!(p.evaluate(&s).is_none());
        let _mv = p.select_move(&s).unwrap();
        let scores = p.evaluate(&s).expect("scores after select_move");
        assert!(!scores.is_empty());
        let total: f32 = scores.values().sum();
        // 正規化されているので合計 ≈ 1.0
        assert!((total - 1.0).abs() < 1e-3, "total = {total}");
        // 全ての値が `0.0..=1.0`
        for &v in scores.values() {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    /// MCTS は Random に対して大幅優位が期待できる ( seed 固定 10 局で勝率 60% 以上)．
    #[test]
    fn beats_random_majority() {
        use crate::Player;
        let trials = 10;
        let mut mcts_wins = 0u32;
        for trial_seed in 0..trials {
            // MCTS は黒，Random は白 ( シードを順次変えて多様性を出す)
            let mut mcts =
                MctsPlayer::new(Color::Black, MctsConfig::new(50)).with_seed(0xABCD ^ trial_seed);
            let mut rnd = RandomPlayer::with_seed(Color::White, 1234 + trial_seed);
            let mut s = GameState::standard_8x8();
            let mut safety = 200u32;
            while !s.is_terminal() && safety > 0 {
                let legal = s.legal_moves();
                let mv = if legal.is_empty() {
                    Move::Pass
                } else if s.side_to_move == Color::Black {
                    mcts.select_move(&s).unwrap()
                } else {
                    rnd.select_move(&s).unwrap()
                };
                if s.apply_move(mv).is_err() {
                    break;
                }
                safety -= 1;
            }
            if let Some(r) = s.result()
                && r.winner == Some(Color::Black)
            {
                mcts_wins += 1;
            }
        }
        assert!(
            mcts_wins >= 6,
            "MCTS should beat Random in >= 60% of games, got {mcts_wins}/{trials}"
        );
    }
}
