//! プロパティテスト: 任意 seed で Random vs Random は必ず 200 手以内に終局する．

use othello_core::prelude::*;
use othello_engine::prelude::*;
use othello_player::RandomPlayer;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn random_vs_random_terminates_within_200_moves(
        seed_b in any::<u64>(),
        seed_w in any::<u64>(),
    ) {
        let mut engine = GameEngine::new(EngineConfig {
            board_size: BoardSize::STANDARD,
            game_id: None,
            max_moves: Some(200),
            log_callback: None,
            jsonl_logger: None,
        }).unwrap();
        let mut b = RandomPlayer::with_seed(Color::Black, seed_b);
        let mut w = RandomPlayer::with_seed(Color::White, seed_w);
        let r = engine.run(&mut b, &mut w).unwrap();
        prop_assert!(r.total_moves <= 200);
        prop_assert!(engine.state().is_terminal());
    }
}
