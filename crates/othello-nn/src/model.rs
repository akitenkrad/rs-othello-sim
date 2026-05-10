//! NN モデル trait と推論結果型．

use crate::error::NnError;
use candle_core::{Device, Tensor};
use othello_core::BoardSize;

/// NN policy/value モデルが満たすべきインターフェース．
///
/// 実装側は forward 1 回で **policy ヘッド ( logits)** と **value ヘッド ( scalar)** の
/// 双方を返すことを期待する．実装が分離している場合は呼び出し側で 2 回 forward すること．
///
/// `Send` 境界はバッチ並列実行 ( `othello_engine::BatchRunner`) で利用するため要求する．
pub trait NnModel: Send {
    /// 推論を実行する．
    ///
    /// - `input`: shape `(B, 3, H, W)` の `f32` テンソル ( own / opp / legal mask)．
    /// - 戻り値: `(policy_logits, value)`．
    ///   - `policy_logits` は `(B, H*W + 1)` ( 末尾は Pass)．
    ///   - `value` は `(B,)` の tanh 空間 ( 自分視点の勝率推定 $[-1, 1]$)．
    fn forward(&self, input: &Tensor) -> Result<(Tensor, Tensor), NnError>;

    /// 期待入力盤面サイズ．forward 入力の shape チェックに使う．
    fn board_size(&self) -> BoardSize;

    /// 動作デバイス ( CPU / Metal / CUDA)．本タスクでは CPU のみ前提．
    fn device(&self) -> &Device;
}

/// 1 局面に対する NN 出力．
///
/// - `policy` は softmax 後の確率分布 ( 合計 1.0)．長さ `H*W + 1` ( 末尾 Pass)．
/// - `value` は自分視点の勝率推定 $[-1, 1]$ ( 1 が勝ち寄り)．
#[derive(Debug, Clone)]
pub struct PolicyValue {
    /// softmax 適用後の確率ベクトル．
    pub policy: Vec<f32>,
    /// tanh 空間の value．
    pub value: f32,
}
