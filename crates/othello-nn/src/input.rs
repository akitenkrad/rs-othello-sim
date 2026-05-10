//! [`GameState`] → Candle [`Tensor`] への変換．
//!
//! 3 channel ( own / opp / legal mask) は [`othello_rl::Observation::Planes`] と同じ
//! レイアウトを使う．本モジュールでは ndarray ↔ Tensor の橋渡しを担う．

use crate::error::NnError;
use candle_core::{DType, Device, Tensor};
use othello_core::{Color, GameState};
use othello_rl::{Observation, ObservationType, make_observation};

/// `state` を `view_color` 視点の `(1, 3, H, W)` テンソルに変換する．
///
/// チャネルは順に own / opp / legal-mask．[`othello_rl::make_observation`] の Planes 表現を
/// そのまま Tensor に流し込んでいるため，OthelloEnv で学習したモデルとの整合性が取れる．
pub fn state_to_tensor(
    state: &GameState,
    view_color: Color,
    device: &Device,
) -> Result<Tensor, NnError> {
    let obs = make_observation(state, view_color, ObservationType::Planes, &[]);
    let array = match obs {
        Observation::Planes(a) => a,
        _ => unreachable!("requested ObservationType::Planes"),
    };
    let shape = array.shape().to_vec();
    debug_assert_eq!(shape.len(), 3);
    let h = shape[1];
    let w = shape[2];
    let data: Vec<f32> = array.into_raw_vec_and_offset().0;
    let t = Tensor::from_vec(data, (1, 3, h, w), device)?.to_dtype(DType::F32)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::{BoardSize, Color, Coord, GameState};

    #[test]
    fn standard_8x8_shape() {
        let s = GameState::standard_8x8();
        let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
        assert_eq!(t.shape().dims(), &[1, 3, 8, 8]);
    }

    #[test]
    fn legal_mask_only_for_side_to_move() {
        // 黒手番 → Black 視点では legal mask が立つ
        let s = GameState::standard_8x8();
        let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
        let mask: Vec<f32> = t
            .get(0)
            .unwrap()
            .get(2)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let sum: f32 = mask.iter().sum();
        assert!((sum - 4.0).abs() < 1e-6, "sum={sum}");

        // White 視点では手番でないので mask は 0
        let t2 = state_to_tensor(&s, Color::White, &Device::Cpu).unwrap();
        let mask2: Vec<f32> = t2
            .get(0)
            .unwrap()
            .get(2)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let sum2: f32 = mask2.iter().sum();
        assert!(sum2.abs() < 1e-6, "sum={sum2}");
    }

    #[test]
    fn own_opp_swapped_when_view_color_changes() {
        let s = GameState::standard_8x8();
        let tb = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
        let tw = state_to_tensor(&s, Color::White, &Device::Cpu).unwrap();

        let own_b: Vec<f32> = tb
            .get(0)
            .unwrap()
            .get(0)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let opp_w: Vec<f32> = tw
            .get(0)
            .unwrap()
            .get(1)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert_eq!(own_b, opp_w);
    }

    #[test]
    fn small_board_4x4_shape() {
        let s = GameState::standard(BoardSize::square(4)).unwrap();
        let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
        assert_eq!(t.shape().dims(), &[1, 3, 4, 4]);
    }

    #[test]
    fn unused_coord_keeps_lint_happy() {
        // Coord をテスト内で使わないと unused_imports 警告が出るので参照する
        let _ = Coord::new(0, 0);
    }
}
