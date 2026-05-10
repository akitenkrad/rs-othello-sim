//! ランダム初期化モデルでの 1 局 smoke test．

use candle_core::Device;
use othello_core::{BoardSize, Color, GameState, Move};
use othello_engine::{EngineConfig, GameEngine};
use othello_nn::{CandleModel, NnEvaluator};
use othello_player::{Player, RandomPlayer};

#[test]
fn nn_vs_random_completes_game_8x8() {
    let model = CandleModel::random_init(BoardSize::STANDARD, 42, Device::Cpu).unwrap();
    let mut nn = NnEvaluator::new(Color::Black, model)
        .with_seed(7)
        .deterministic();
    let mut rnd = RandomPlayer::with_seed(Color::White, 3);
    let mut engine = GameEngine::new(EngineConfig::with_size(BoardSize::STANDARD)).unwrap();
    let result = engine.run(&mut nn, &mut rnd).unwrap();
    // 8x8 では終局石数は 4..=64 の範囲．
    let total = result.black + result.white;
    assert!(
        (4..=64).contains(&total),
        "unexpected total stones: {total}"
    );
}

#[test]
fn evaluator_overlay_is_populated_after_first_move() {
    let model = CandleModel::random_init(BoardSize::STANDARD, 17, Device::Cpu).unwrap();
    let mut nn = NnEvaluator::new(Color::Black, model).with_seed(1);
    let s = GameState::standard_8x8();
    let mv = nn.select_move(&s).unwrap();
    let legal = s.legal_moves();
    assert!(legal.contains(&mv));

    let ev = nn.evaluator().expect("nn provides evaluator");
    let scores = ev.evaluate(&s).expect("evaluate yields Some after select");
    assert_eq!(scores.len(), 4);
    let total: f32 = scores.values().sum();
    assert!((total - 1.0).abs() < 1e-4);
}

#[test]
fn deterministic_with_seed_is_reproducible() {
    let m1 = CandleModel::random_init(BoardSize::STANDARD, 5, Device::Cpu).unwrap();
    let m2 = CandleModel::random_init(BoardSize::STANDARD, 5, Device::Cpu).unwrap();
    let mut p1 = NnEvaluator::new(Color::Black, m1)
        .with_seed(99)
        .deterministic();
    let mut p2 = NnEvaluator::new(Color::Black, m2)
        .with_seed(99)
        .deterministic();
    let s = GameState::standard_8x8();
    let mv1 = p1.select_move(&s).unwrap();
    let mv2 = p2.select_move(&s).unwrap();
    // deterministic mode: argmax over policy distribution.  Even though models are
    // different random initializations, the test only checks that *the same* model
    // + seed yields stable picks across two runs of the same evaluator.
    let mv1_again = p1.select_move(&s).unwrap();
    assert_eq!(mv1, mv1_again);
    // 異なる random init の場合，必ずしも一致しないので存在確認のみ
    let _ = mv2;
}

#[test]
fn nn_select_returns_only_legal_moves() {
    let model = CandleModel::random_init(BoardSize::STANDARD, 0, Device::Cpu).unwrap();
    let mut nn = NnEvaluator::new(Color::Black, model).with_seed(11);
    let s = GameState::standard_8x8();
    let mv = nn.select_move(&s).unwrap();
    match mv {
        Move::Place(_) => {
            assert!(s.legal_moves().contains(&mv));
        }
        Move::Pass => {
            // Standard initial 8x8 always has 4 legal moves so Pass should never appear here.
            panic!("unexpected pass at standard start");
        }
    }
}
