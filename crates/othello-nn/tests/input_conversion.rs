//! [`state_to_tensor`] のスキーマ整合性テスト．

use candle_core::Device;
use othello_core::{BoardSize, Color, Coord, GameState, Move};
use othello_nn::state_to_tensor;

#[test]
fn shape_8x8() {
    let s = GameState::standard_8x8();
    let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
    assert_eq!(t.shape().dims(), &[1, 3, 8, 8]);
}

#[test]
fn shape_4x4() {
    let s = GameState::standard(BoardSize::square(4)).unwrap();
    let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
    assert_eq!(t.shape().dims(), &[1, 3, 4, 4]);
}

#[test]
fn shape_6x6() {
    let s = GameState::standard(BoardSize::square(6)).unwrap();
    let t = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
    assert_eq!(t.shape().dims(), &[1, 3, 6, 6]);
}

#[test]
fn view_color_swaps_planes() {
    let s = GameState::standard_8x8();
    let tb = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
    let tw = state_to_tensor(&s, Color::White, &Device::Cpu).unwrap();
    let own_b = tb
        .get(0)
        .unwrap()
        .get(0)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
    let opp_w = tw
        .get(0)
        .unwrap()
        .get(1)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
    assert_eq!(own_b, opp_w, "own(Black) should equal opp(White)");
}

#[test]
fn legal_mask_only_for_side_to_move() {
    let s = GameState::standard_8x8();
    let tb = state_to_tensor(&s, Color::Black, &Device::Cpu).unwrap();
    let mask = tb
        .get(0)
        .unwrap()
        .get(2)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
    let sum: f32 = mask.iter().sum();
    assert!((sum - 4.0).abs() < 1e-6);

    let tw = state_to_tensor(&s, Color::White, &Device::Cpu).unwrap();
    let mask_w = tw
        .get(0)
        .unwrap()
        .get(2)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();
    let sum_w: f32 = mask_w.iter().sum();
    assert!(sum_w.abs() < 1e-6);
}

#[test]
fn coord_module_in_scope() {
    // Coord は input crate からは公開されていないが，cell 値は同じ規約に従う
    let c = Coord::new(3, 4);
    let m = Move::Place(c);
    if let Move::Place(co) = m {
        assert_eq!(co.row, 3);
        assert_eq!(co.col, 4);
    }
}
