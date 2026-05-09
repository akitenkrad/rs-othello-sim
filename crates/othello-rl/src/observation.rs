//! 観測 ( [`Observation`]) の表現．
//!
//! - [`Observation::Planes`] — 3 channels (own / opp / legal mask)，shape `[3, H, W]`
//! - [`Observation::Flat`] — `Planes` を flatten したベクトル，shape `[3*H*W]`
//! - [`Observation::MoveSequence`] — 手順序列 ( 1-indexed `1..=H*W`，Pass は 0)

use ndarray::{Array1, Array3};
use othello_core::{BoardSize, Color, Coord, GameState, Move};
use serde::{Deserialize, Serialize};

/// 観測形式の選択．
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    /// `[3, H, W]` の `f32` テンソル．
    Planes,
    /// `[3*H*W]` の `f32` ベクトル．
    Flat,
    /// 手順序列 ( 1-indexed)．Pass は 0．
    MoveSequence,
}

/// 観測．
#[derive(Debug, Clone)]
pub enum Observation {
    /// `[3, H, W]` テンソル．
    Planes(Array3<f32>),
    /// flatten ベクトル．
    Flat(Array1<f32>),
    /// 手順序列 ( 1-indexed)．
    MoveSequence(Vec<u16>),
}

impl Observation {
    /// shape を返す ( デバッグ用)．
    #[must_use]
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Self::Planes(a) => a.shape().to_vec(),
            Self::Flat(a) => a.shape().to_vec(),
            Self::MoveSequence(v) => vec![v.len()],
        }
    }
}

/// `state` に対する `view_color` 視点の観測を `ty` 形式で生成する．
///
/// `history` は `MoveSequence` のときに参照される ( 着手列)．
/// `Planes` / `Flat` では使用しない．
#[must_use]
pub fn make_observation(
    state: &GameState,
    view_color: Color,
    ty: ObservationType,
    history: &[Move],
) -> Observation {
    let size = state.board.size();
    match ty {
        ObservationType::Planes => Observation::Planes(make_planes(state, view_color, size)),
        ObservationType::Flat => {
            let planes = make_planes(state, view_color, size);
            let total = planes.len();
            let flat = planes
                .into_shape_with_order((total,))
                .expect("planes are contiguous");
            Observation::Flat(flat)
        }
        ObservationType::MoveSequence => {
            Observation::MoveSequence(make_move_sequence(history, size))
        }
    }
}

fn make_planes(state: &GameState, view_color: Color, size: BoardSize) -> Array3<f32> {
    let h = size.rows as usize;
    let w = size.cols as usize;
    let mut arr = Array3::<f32>::zeros((3, h, w));
    let opp_color = view_color.opponent();
    // legal mask は「view_color が手番のときのみ」立てる．
    let legal_for_view = if state.side_to_move == view_color {
        state.legal_moves()
    } else {
        Vec::new()
    };
    for r in 0..size.rows {
        for c in 0..size.cols {
            let coord = Coord::new(r, c);
            match state.board.cell(coord) {
                Some(col) if col == view_color => {
                    arr[[0, r as usize, c as usize]] = 1.0;
                }
                Some(col) if col == opp_color => {
                    arr[[1, r as usize, c as usize]] = 1.0;
                }
                _ => {}
            }
        }
    }
    for mv in legal_for_view {
        if let Move::Place(coord) = mv {
            arr[[2, coord.row as usize, coord.col as usize]] = 1.0;
        }
    }
    arr
}

fn make_move_sequence(history: &[Move], size: BoardSize) -> Vec<u16> {
    history
        .iter()
        .map(|m| match m {
            Move::Place(c) => 1 + (c.row as u16) * (size.cols as u16) + (c.col as u16),
            Move::Pass => 0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planes_shape_8x8() {
        let s = GameState::standard_8x8();
        let obs = make_observation(&s, Color::Black, ObservationType::Planes, &[]);
        assert_eq!(obs.shape(), vec![3, 8, 8]);
    }

    #[test]
    fn flat_shape_8x8() {
        let s = GameState::standard_8x8();
        let obs = make_observation(&s, Color::Black, ObservationType::Flat, &[]);
        assert_eq!(obs.shape(), vec![3 * 8 * 8]);
    }

    #[test]
    fn legal_mask_only_when_agent_to_move() {
        let s = GameState::standard_8x8();
        // 黒の手番なので Black 視点では mask が 4 つ立っている
        let obs = make_observation(&s, Color::Black, ObservationType::Planes, &[]);
        if let Observation::Planes(a) = obs {
            let mask_sum: f32 = a.slice(ndarray::s![2, .., ..]).sum();
            assert!(
                (mask_sum - 4.0).abs() < 1e-6,
                "expected 4 legal mask cells, got {mask_sum}"
            );
        } else {
            panic!("expected Planes");
        }
        // White 視点では手番でないので mask は 0
        let obs = make_observation(&s, Color::White, ObservationType::Planes, &[]);
        if let Observation::Planes(a) = obs {
            let mask_sum: f32 = a.slice(ndarray::s![2, .., ..]).sum();
            assert!(mask_sum.abs() < 1e-6);
        } else {
            panic!("expected Planes");
        }
    }

    #[test]
    fn move_sequence_encoding() {
        let size = BoardSize::STANDARD;
        let history = vec![
            Move::Place(Coord::new(0, 0)),
            Move::Pass,
            Move::Place(Coord::new(1, 2)),
        ];
        let seq = make_move_sequence(&history, size);
        // Place(1, 2) -> 1 + (1*8) + 2 = 11
        assert_eq!(seq, vec![1, 0, 11]);
    }

    #[test]
    fn own_opp_planes_are_distinct() {
        let s = GameState::standard_8x8();
        let obs = make_observation(&s, Color::Black, ObservationType::Planes, &[]);
        if let Observation::Planes(a) = obs {
            // 黒 2 個・白 2 個が初期配置
            let own: f32 = a.slice(ndarray::s![0, .., ..]).sum();
            let opp: f32 = a.slice(ndarray::s![1, .., ..]).sum();
            assert!((own - 2.0).abs() < 1e-6);
            assert!((opp - 2.0).abs() < 1e-6);
        }
    }
}
