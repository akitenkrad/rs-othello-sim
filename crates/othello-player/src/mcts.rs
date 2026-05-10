//! [`MctsPlayer`]: an MCTS player based on UCT (Upper Confidence Bound
//! applied to Trees).
//!
//! Minimal implementation of standard UCT. By default the tree is rebuilt
//! before every move selection. Setting [`MctsConfig::tree_reuse`] to
//! `true` carries over the subtree under "the move I played followed by
//! the opponent's reply" so that visit counts and win-rate statistics are
//! preserved (introduced in Phase 6.2). The default `false` matches the
//! original behavior.
//!
//! ## Algorithm
//!
//! 1. **Selection**: descend to a leaf via
//!    `argmax(Q/N + c * sqrt(ln(N_parent) / N))`.
//! 2. **Expansion**: expand one unexpanded legal move.
//! 3. **Simulation (rollout)**: play random legal moves to terminal, up
//!    to `max_rollout_depth`.
//! 4. **Backpropagation**: backpropagate the terminal result
//!    (win/loss/draw -> +1/-1/0).
//!
//! ## Example
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

/// Configuration for the MCTS player.
#[derive(Debug, Clone, Copy)]
pub struct MctsConfig {
    /// Number of simulations per move.
    pub simulations: u32,
    /// UCT exploration constant $c$. Typically $\sqrt{2} \approx 1.414$.
    pub exploration: f64,
    /// Random seed (`None` derives from OS randomness).
    pub seed: Option<u64>,
    /// Maximum rollout depth (a safety cap).
    pub max_rollout_depth: u32,
    /// Whether to reuse the search tree across moves.
    ///
    /// When `true`, [`MctsPlayer::select_move`] reuses the subtree under
    /// "the move I played followed by the opponent's reply" as the new
    /// root, carrying over visit counts and win-rate statistics to save
    /// search work.
    ///
    /// **Defaults to `false`**, which matches the original behavior of
    /// rebuilding the tree on every move.
    pub tree_reuse: bool,
}

impl MctsConfig {
    /// Builds a config with only `simulations` set; other fields use
    /// defaults.
    ///
    /// - `exploration = sqrt(2)`
    /// - `seed = None`
    /// - `max_rollout_depth = 200`
    /// - `tree_reuse = false` (original behavior).
    #[must_use]
    pub fn new(simulations: u32) -> Self {
        Self {
            simulations,
            exploration: std::f64::consts::SQRT_2,
            seed: None,
            max_rollout_depth: 200,
            tree_reuse: false,
        }
    }

    /// Builder: sets `seed`.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Builder: sets `exploration`.
    #[must_use]
    pub fn with_exploration(mut self, c: f64) -> Self {
        self.exploration = c;
        self
    }

    /// Builder: sets `max_rollout_depth`.
    #[must_use]
    pub fn with_max_rollout_depth(mut self, depth: u32) -> Self {
        self.max_rollout_depth = depth;
        self
    }

    /// Builder: sets `tree_reuse`.
    ///
    /// When `true`, inherits the subtree from the previous search to
    /// reduce the cost of subsequent searches. Defaults to `false`.
    #[must_use]
    pub fn with_tree_reuse(mut self, enable: bool) -> Self {
        self.tree_reuse = enable;
        self
    }
}

impl Default for MctsConfig {
    /// Equivalent to [`MctsConfig::new`] except `simulations = 1000`
    /// (so `tree_reuse = false`).
    fn default() -> Self {
        Self::new(1000)
    }
}

/// MCTS player based on UCT.
///
/// By default the tree is rebuilt before every move selection. Setting
/// [`MctsConfig::tree_reuse`] to `true` reuses the subtree under "the
/// move I played followed by the opponent's reply" (tree reuse).
pub struct MctsPlayer {
    name: String,
    color: Color,
    config: MctsConfig,
    rng: ChaCha8Rng,
    /// `(move directly under the root, visit count, Q value)` collected
    /// in the most recent [`select_move`] / [`Evaluator::evaluate`] call.
    ///
    /// Kept so that `Evaluator` can return normalized values. Intermediate
    /// states during `select_move` are not reflected (only updated on
    /// completion).
    last_root_stats: Vec<(Move, u32, f64)>,
    /// Subtree retained after the most recent `select_move`. Only updated
    /// when `tree_reuse=true`.
    ///
    /// The subtree's root corresponds to "the position immediately after
    /// applying the move I selected"; its children start from the
    /// opponent's reply.
    persisted_tree: Option<MctsTree>,
    /// State corresponding to the root of the `persisted_tree` above
    /// (used to verify a match).
    persisted_root_state: Option<GameState>,
    /// Total visit count at the root after the most recent `select_move`
    /// (used for debugging and tests).
    last_root_visits: u32,
}

impl MctsPlayer {
    /// Constructs a player with the given `color` and `config`.
    #[must_use]
    pub fn new(color: Color, config: MctsConfig) -> Self {
        let seed = config.seed.unwrap_or_else(default_seed);
        Self {
            name: "MctsPlayer".to_string(),
            color,
            config,
            rng: ChaCha8Rng::seed_from_u64(seed),
            last_root_stats: Vec::new(),
            persisted_tree: None,
            persisted_root_state: None,
            last_root_visits: 0,
        }
    }

    /// Builder: sets the seed after construction. Overrides
    /// `MctsConfig::seed`.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self.rng = ChaCha8Rng::seed_from_u64(seed);
        self
    }

    /// Builder: changes the player name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Returns a reference to the configuration.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &MctsConfig {
        &self.config
    }

    /// Returns the visit count accumulated at the root after the most
    /// recent `select_move` (useful for tests and diagnostics).
    ///
    /// With `tree_reuse = false` this typically equals the current
    /// `simulations`. With `tree_reuse = true` it can be larger because
    /// the subtree from the previous search is inherited. Returns 0 when
    /// `select_move` has never been called.
    #[inline]
    #[must_use]
    pub fn last_root_visit_total(&self) -> u32 {
        self.last_root_visits
    }

    /// Extracts the subtree corresponding to the current `state` from the
    /// previous search tree (tree reuse).
    ///
    /// # Algorithm (Strategy 1: move-indexed lookup)
    ///
    /// 1. `persisted_root_state` is the position that was reached
    ///    immediately after applying our previously chosen move.
    /// 2. `state.last_move` is expected to be the opponent's reply (the
    ///    move that the game engine applied most recently).
    /// 3. From the root of `persisted_tree`, look up the child keyed by
    ///    `state.last_move` and return the subtree rooted at that child.
    /// 4. If the looked-up child's position does not match the current
    ///    `state`, return `None` (fresh-tree fallback).
    fn try_reuse_tree(&mut self, state: &GameState) -> Option<MctsTree> {
        let prev_tree = self.persisted_tree.take()?;
        let _prev_root_state = self.persisted_root_state.take()?;
        let opp_mv = state.last_move?;

        // prev_tree の root から opp_mv に対応する子を探す．
        let mismatch = match prev_tree.child_state(opp_mv) {
            None => {
                tracing::debug!(
                    target: "othello_player::mcts",
                    "tree_reuse: child for opp_mv not present in persisted tree; rebuilding"
                );
                true
            }
            Some(child_state) => {
                // 子の局面と現在の state を比較して一致を確認．
                let same = child_state.board == state.board
                    && child_state.side_to_move == state.side_to_move;
                if !same {
                    tracing::warn!(
                        target: "othello_player::mcts",
                        "tree_reuse: state mismatch after applying opponent reply; falling back to fresh tree"
                    );
                }
                !same
            }
        };
        if mismatch {
            return None;
        }
        prev_tree.into_subtree(opp_mv)
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
            self.last_root_visits = 0;
            // tree_reuse でも Pass は単一遷移なので木を保持しても意味が薄いが，
            // 次回呼び出し時の整合性確認用にクリアする ( 古い state との不一致を避ける)．
            if self.config.tree_reuse {
                self.persisted_tree = None;
                self.persisted_root_state = None;
            }
            return Ok(Move::Pass);
        }
        if legal.len() == 1 {
            self.last_root_stats = vec![(legal[0], 1, 0.0)];
            self.last_root_visits = 1;
            // 1 手しかない場合も木を作らないので reuse 連鎖を切る．
            if self.config.tree_reuse {
                self.persisted_tree = None;
                self.persisted_root_state = None;
            }
            return Ok(legal[0]);
        }

        // 1) 木の準備 ( 新規 or 部分木継承)．
        let mut tree = if self.config.tree_reuse {
            self.try_reuse_tree(state)
                .unwrap_or_else(|| MctsTree::new(state.clone(), self.config.exploration))
        } else {
            MctsTree::new(state.clone(), self.config.exploration)
        };

        // 2) シミュレーション．
        for _ in 0..self.config.simulations {
            tree.run_one(&mut self.rng, self.config.max_rollout_depth);
        }

        // 3) ルート統計と best move を取得．
        self.last_root_stats = tree.root_stats();
        self.last_root_visits = tree.root_visits();
        let best = tree.best_move().ok_or_else(|| {
            PlayerError::Other("MctsPlayer: tree returned no move ( unexpected)".into())
        })?;

        // 4) tree_reuse 有効時は，自分の選択手を適用した直後の部分木を保存する．
        if self.config.tree_reuse {
            if let Some((subtree, post_state)) = tree.extract_subtree(best) {
                self.persisted_tree = Some(subtree);
                self.persisted_root_state = Some(post_state);
            } else {
                self.persisted_tree = None;
                self.persisted_root_state = None;
            }
        }

        Ok(best)
    }

    fn reset(&mut self) {
        if let Some(seed) = self.config.seed {
            self.rng = ChaCha8Rng::seed_from_u64(seed);
        }
        self.last_root_stats.clear();
        self.persisted_tree = None;
        self.persisted_root_state = None;
        self.last_root_visits = 0;
    }

    fn evaluator(&mut self) -> Option<&mut dyn Evaluator> {
        Some(self)
    }
}

impl Evaluator for MctsPlayer {
    /// Returns the normalized visit counts of the children of the most
    /// recent `select_move` root.
    ///
    /// If the supplied `state` differs from the most recent root (stale
    /// history), the implementation simply returns the current cache (call
    /// `select_move` first if up-to-date data is required). Returns `None`
    /// when no visits were recorded or the cache has not yet been built.
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

/// Statistics and untried-move queue associated with a node.
#[derive(Debug)]
struct MctsNode {
    /// Position corresponding to this node.
    state: GameState,
    /// Parent node (`None` for the root).
    parent: Option<usize>,
    /// Move that led from the parent to this node. Kept for debugging and
    /// future extensions.
    #[allow(dead_code)]
    incoming_move: Option<Move>,
    /// Cumulative score from this node's perspective (`win = +1`,
    /// `draw = 0`, `loss = -1`). More precisely, this stores the rollout
    /// result rebased to the perspective of the side **about to move at
    /// this node**, rather than the `agent_color` perspective at rollout
    /// termination (the standard UCT convention).
    score_sum: f64,
    /// Visit count.
    visits: u32,
    /// Legal moves that have not yet been expanded.
    untried_moves: Vec<Move>,
    /// Child nodes `(move, node_id)`.
    children: Vec<(Move, usize)>,
    /// Whether this is a terminal node (the game has ended).
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

    /// Descends to a leaf via UCT and returns the leaf node id.
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

    /// Adds a single child to the leaf and returns the new child's id.
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

    /// Rollout. Returns the result from the perspective of the side about
    /// to move at the root (`+1 / 0 / -1`).
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

    /// Backpropagates the rollout result `result_for_root` (`+1 / 0 / -1`
    /// from the perspective of the side about to move at the root).
    ///
    /// Each node accumulates the score from the perspective of the side
    /// that made the move into it (i.e., the parent's `side_to_move`).
    /// Thanks to this convention, child selection only has to maximize the
    /// win rate from the parent's perspective.
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

    /// Returns `(move, visits, q)` for each child of the root, with
    /// `q = score_sum / visits`.
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

    /// Selects the most-visited child of the root.
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

    /// Cumulative visit count at the root (sum of all children's visits
    /// plus the leaf-rollout backups).
    fn root_visits(&self) -> u32 {
        self.nodes[0].visits
    }

    /// Returns the position of the root's child corresponding to `mv`
    /// (`None` if no such child exists).
    fn child_state(&self, mv: Move) -> Option<&GameState> {
        let root = &self.nodes[0];
        for &(m, child_id) in &root.children {
            if m == mv {
                return Some(&self.nodes[child_id].state);
            }
        }
        None
    }

    /// Extracts the subtree reached by applying our chosen move
    /// `chosen_move` (used for tree reuse).
    ///
    /// If the matching child exists, returns the [`MctsTree`] rebuilt with
    /// that child as the new root together with the corresponding
    /// [`GameState`]. Returns `None` otherwise.
    fn extract_subtree(self, chosen_move: Move) -> Option<(MctsTree, GameState)> {
        let root = &self.nodes[0];
        let mut child_id_opt: Option<usize> = None;
        for &(m, cid) in &root.children {
            if m == chosen_move {
                child_id_opt = Some(cid);
                break;
            }
        }
        let child_id = child_id_opt?;
        let post_state = self.nodes[child_id].state.clone();
        let subtree = self.into_subtree_inner(child_id)?;
        Some((subtree, post_state))
    }

    /// Returns the subtree whose new root is the child matching `mv`
    /// (used for tree reuse).
    ///
    /// Used by internal callers; returns `None` if no such child exists.
    fn into_subtree(self, mv: Move) -> Option<MctsTree> {
        let root = &self.nodes[0];
        let mut child_id_opt: Option<usize> = None;
        for &(m, cid) in &root.children {
            if m == mv {
                child_id_opt = Some(cid);
                break;
            }
        }
        let child_id = child_id_opt?;
        self.into_subtree_inner(child_id)
    }

    /// Shared implementation that carves out a subtree rooted at
    /// `new_root_id`.
    ///
    /// In the old tree, `score_sum` is recorded from the perspective of
    /// the parent node's `side_to_move`. The new root has no parent, so
    /// its viewpoint switches to `root_side = root.state.side_to_move`.
    /// When the old parent's `side_to_move` differs from the new root's
    /// `side_to_move` (the usual case), the sign of the new root's
    /// `score_sum` is flipped to keep things consistent. All other nodes
    /// retain the same parent (only the parent of the parent might have
    /// changed), so their viewpoint is unchanged.
    fn into_subtree_inner(self, new_root_id: usize) -> Option<MctsTree> {
        let MctsTree { nodes, exploration } = self;

        // 新 root の score_sum 視点補正用フラグ．
        let new_root_side = nodes[new_root_id].state.side_to_move;
        let new_root_flip = match nodes[new_root_id].parent {
            Some(p) => nodes[p].state.side_to_move != new_root_side,
            None => false, // 既に root だった場合はそのまま
        };

        // BFS で new_root_id 配下のノード id を収集し，旧 id → 新 id のマップを作る．
        let mut old_to_new: HashMap<usize, usize> = HashMap::new();
        let mut order: Vec<usize> = Vec::new();
        let mut queue: Vec<usize> = vec![new_root_id];
        old_to_new.insert(new_root_id, 0);
        order.push(new_root_id);
        while let Some(cur) = queue.pop() {
            for &(_mv, child) in &nodes[cur].children {
                if !old_to_new.contains_key(&child) {
                    let new_id = old_to_new.len();
                    old_to_new.insert(child, new_id);
                    order.push(child);
                    queue.push(child);
                }
            }
        }

        // 新ノード列を組み立てる．`order` 順にプッシュするのでインデックスは連続する
        // ( old_to_new[order[i]] == i)．
        let mut new_nodes: Vec<MctsNode> = Vec::with_capacity(order.len());
        for &old_id in &order {
            let n = &nodes[old_id];
            let new_parent = if old_id == new_root_id {
                None
            } else {
                n.parent.and_then(|p| old_to_new.get(&p).copied())
            };
            let new_children: Vec<(Move, usize)> = n
                .children
                .iter()
                .filter_map(|&(mv, cid)| old_to_new.get(&cid).map(|&new_cid| (mv, new_cid)))
                .collect();
            let score_sum = if old_id == new_root_id && new_root_flip {
                -n.score_sum
            } else {
                n.score_sum
            };
            new_nodes.push(MctsNode {
                state: n.state.clone(),
                parent: new_parent,
                incoming_move: if old_id == new_root_id {
                    None
                } else {
                    n.incoming_move
                },
                score_sum,
                visits: n.visits,
                untried_moves: n.untried_moves.clone(),
                children: new_children,
                terminal: n.terminal,
            });
        }
        Some(MctsTree {
            nodes: new_nodes,
            exploration,
        })
    }
}

/// Score from the `view` perspective at a terminal or cut-off state
/// (`+1` win, `0` draw, `-1` loss).
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

    // ===== Phase 6.2: tree_reuse のテスト =====

    #[test]
    fn tree_reuse_default_false() {
        // 後方互換: 既存 API ( new / Default) では tree_reuse は false
        assert!(!MctsConfig::new(100).tree_reuse);
        assert!(!MctsConfig::default().tree_reuse);
    }

    #[test]
    fn tree_reuse_with_builder_sets_flag() {
        let cfg = MctsConfig::new(100).with_tree_reuse(true);
        assert!(cfg.tree_reuse);
        let cfg2 = cfg.with_tree_reuse(false);
        assert!(!cfg2.tree_reuse);
    }

    /// Confirms that, with many simulations and many games, `tree_reuse =
    /// true` and `tree_reuse = false` do not produce dramatically
    /// different win rates (i.e., they correspond to the same policy).
    #[test]
    fn tree_reuse_does_not_change_decision_distribution() {
        use crate::Player;
        let trials = 6;
        let mut wins_off = 0u32;
        let mut wins_on = 0u32;
        for trial in 0..trials {
            for &reuse in &[false, true] {
                let mut mcts =
                    MctsPlayer::new(Color::Black, MctsConfig::new(150).with_tree_reuse(reuse))
                        .with_seed(0x1234 ^ trial);
                let mut rnd = RandomPlayer::with_seed(Color::White, 5678 + trial);
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
                    if reuse {
                        wins_on += 1;
                    } else {
                        wins_off += 1;
                    }
                }
            }
        }
        // 6 局中の勝率差が ±2 局以内なら同等水準とみなす．
        let diff = (wins_on as i32 - wins_off as i32).abs();
        assert!(
            diff <= 2,
            "tree_reuse should not change strength dramatically: wins_off={wins_off}, wins_on={wins_on}"
        );
    }

    /// With `tree_reuse = true`, the root visit total at the second and
    /// later moves should exceed the first move's total (evidence that
    /// the subtree is being inherited).
    ///
    /// Choosing a larger `sims` ensures that depth-2 nodes (i.e., the
    /// position after the opponent's reply) are expanded at least once,
    /// guaranteeing that the subtree-inheritance path is exercised.
    #[test]
    fn tree_reuse_actually_amortizes_visits() {
        use crate::Player;
        let sims = 400u32;
        let mut mcts = MctsPlayer::new(Color::Black, MctsConfig::new(sims).with_tree_reuse(true))
            .with_seed(0xABCD);
        let mut rnd = RandomPlayer::with_seed(Color::White, 0x1357);
        let mut s = GameState::standard_8x8();

        // 1 手目: 黒
        let mv1 = mcts.select_move(&s).unwrap();
        let visits_after_move1 = mcts.last_root_visit_total();
        s.apply_move(mv1).unwrap();
        // 1 手目は新規ツリーから始まるので simulations と一致するはず
        assert_eq!(visits_after_move1, sims);

        // 白の応手 ( ランダム)
        let mv_w = rnd.select_move(&s).unwrap();
        s.apply_move(mv_w).unwrap();

        // 2 手目: 黒．ここで部分木継承が効くはず
        let _mv2 = mcts.select_move(&s).unwrap();
        let visits_after_move2 = mcts.last_root_visit_total();
        // 継承後 simulations 回足されるので，1 手目の simulations より大きくなる．
        assert!(
            visits_after_move2 > sims,
            "tree_reuse should amortise visits across moves: after_move1={}, after_move2={}",
            visits_after_move1,
            visits_after_move2
        );
    }

    /// `reset()` clears the persisted tree-reuse state.
    #[test]
    fn tree_reuse_reset_clears_persisted_tree() {
        use crate::Player;
        let sims = 50u32;
        let mut mcts =
            MctsPlayer::new(Color::Black, MctsConfig::new(sims).with_tree_reuse(true)).with_seed(7);
        let s = GameState::standard_8x8();
        let _ = mcts.select_move(&s).unwrap();
        assert!(mcts.persisted_tree.is_some());
        mcts.reset();
        assert!(mcts.persisted_tree.is_none());
        assert!(mcts.persisted_root_state.is_none());
        // reset 直後に再度選んでも問題なく動くこと．
        let _ = mcts.select_move(&s).unwrap();
        // 1 手目 ( reset 直後) は fresh tree なので simulations と一致．
        assert_eq!(mcts.last_root_visit_total(), sims);
    }

    /// Falls back to a fresh tree when the supplied `state` is completely
    /// disconnected from the previous continuation (for example, when the
    /// game engine starts a new game).
    #[test]
    fn tree_reuse_falls_back_on_state_mismatch() {
        use crate::Player;
        let sims = 50u32;
        let mut mcts = MctsPlayer::new(Color::Black, MctsConfig::new(sims).with_tree_reuse(true))
            .with_seed(11);
        let s0 = GameState::standard_8x8();

        // 1 局目を 1 手指し，persisted_tree を作る．
        let mv1 = mcts.select_move(&s0).unwrap();
        let mut s_after_mv1 = s0.clone();
        s_after_mv1.apply_move(mv1).unwrap();
        // 白の手番に進めるが，「相手の応手」を木の合法手と整合しないものに差し替える
        // ことで mismatch を作るのは難しいので，代わりに「reset 後に新しい初期局面」を
        // 与えて fresh tree が作られることを確認する．
        // ( reset で persisted がクリアされるのは別テスト．ここは last_move=None の場合)
        let s_fresh = GameState::standard_8x8();
        // last_move が None のとき try_reuse_tree は None を返すはず
        let _mv = mcts.select_move(&s_fresh).unwrap();
        // 累積訪問数が今回の simulations と一致 ( = fresh tree から始めた)
        assert_eq!(mcts.last_root_visit_total(), sims);
    }

    /// MCTS is expected to dominate Random by a wide margin (at least
    /// 60% wins over 10 fixed-seed games).
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
