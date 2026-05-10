//! [`Transition`] と [`TransitionBatch`] 型定義．
//!
//! `Transition` は self-play / RL の 1 ステップ分の経験を保持する．
//! `TransitionBatch` はサンプリング時にまとめて返すバッチ表現．

use ndarray::{Array1, Array2, Array3, Array4};
use othello_core::Color;
use serde::{Deserialize, Serialize};

/// 1 ステップ分の経験 ( transition)．
///
/// - `observation` : Planes 形式の観測テンソル `(3, H, W)`．`own / opp / legal_mask` を含む．
/// - `action` : `0..H*W+1` の整数 ( 最後が Pass)．
/// - `policy` : AlphaZero 風の visit-count 分布 ( 任意)．長さ `H*W+1`．
/// - `value` : 自分視点の終局報酬 ( -1 / 0 / +1) または TD ターゲット．
/// - `legal_mask` : 観測時点の合法手マスク．長さ `H*W+1`．
/// - `side` : この transition の手番 ( 観測の視点)．
/// - `move_number` : ゲーム内での累計手数．
/// - `game_id` : 由来ゲームの識別子 ( デバッグ用)．
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// `(3, H, W)` Planes 観測．
    pub observation: Array3<f32>,
    /// 行動インデックス．
    pub action: u32,
    /// 行動分布ターゲット ( 任意)．
    pub policy: Option<Array1<f32>>,
    /// 価値ターゲット ( 自分視点)．
    pub value: f32,
    /// 観測時点の合法手マスク．
    pub legal_mask: Array1<bool>,
    /// 観測の視点プレイヤー．
    pub side: Color,
    /// ゲーム内手数．
    pub move_number: u32,
    /// 由来ゲームの識別子．
    pub game_id: String,
}

/// `(B, ...)` バッチ取り出し用の構造．
///
/// Python 側で numpy として stack して受け取ることを想定する．
#[derive(Debug, Clone)]
pub struct TransitionBatch {
    /// `(B, 3, H, W)` の観測テンソル．
    pub observations: Array4<f32>,
    /// `(B,)` の行動インデックス．
    pub actions: Array1<u32>,
    /// `(B, H*W+1)` の行動分布 ( 任意)．バッチ全件で揃っているときのみ `Some`．
    pub policies: Option<Array2<f32>>,
    /// `(B,)` の価値ターゲット．
    pub values: Array1<f32>,
    /// `(B, H*W+1)` の合法手マスク．
    pub legal_masks: Array2<bool>,
    /// PER の優先度更新で使う各 transition の buffer index．
    pub indices: Vec<usize>,
    /// `(B,)` の importance-sampling weight ( PER のみ)．
    pub weights: Option<Array1<f32>>,
}

impl TransitionBatch {
    /// バッチサイズ．
    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// バッチが空かどうか．
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
