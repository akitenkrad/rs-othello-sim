//! Communication-level integration tests for the external-engine player.
//!
//! To run on CI machines without real Edax / Egaroucid binaries, the
//! tests round-trip with a bash-script mock engine. Unix-only (requires
//! bash).

#![cfg(unix)]

use othello_core::{BoardSize, Color, Coord, GameState, Move};
use othello_player::{ExternalEngineConfig, ExternalEnginePlayer, Player, Protocol};
use std::path::PathBuf;
use std::time::Duration;

fn mock_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}

#[test]
fn external_gtp_returns_mocked_move() {
    let cfg = ExternalEngineConfig {
        command: mock_path("mock_engine_gtp.sh"),
        args: vec![],
        protocol: Protocol::Gtp,
        board_size: BoardSize::STANDARD,
        timeout: Duration::from_secs(5),
        working_dir: None,
        env: vec![],
    };
    let mut p = ExternalEnginePlayer::new(Color::Black, cfg);
    let s = GameState::standard_8x8();
    let mv = p.select_move(&s).expect("engine returned move");
    assert_eq!(mv, Move::Place(Coord::new(2, 3))); // D3
}

#[test]
fn external_ntest_returns_mocked_move() {
    let cfg = ExternalEngineConfig {
        command: mock_path("mock_engine_ntest.sh"),
        args: vec![],
        protocol: Protocol::Ntest,
        board_size: BoardSize::STANDARD,
        timeout: Duration::from_secs(5),
        working_dir: None,
        env: vec![],
    };
    let mut p = ExternalEnginePlayer::new(Color::Black, cfg);
    let s = GameState::standard_8x8();
    let mv = p.select_move(&s).expect("engine returned move");
    assert_eq!(mv, Move::Place(Coord::new(2, 3)));
}

#[test]
fn external_reset_reinitializes_engine() {
    let cfg = ExternalEngineConfig {
        command: mock_path("mock_engine_gtp.sh"),
        args: vec![],
        protocol: Protocol::Gtp,
        board_size: BoardSize::STANDARD,
        timeout: Duration::from_secs(5),
        working_dir: None,
        env: vec![],
    };
    let mut p = ExternalEnginePlayer::new(Color::Black, cfg);
    let s = GameState::standard_8x8();
    let _ = p.select_move(&s).unwrap();
    p.reset();
    // reset 後も別のゲームとして select_move が成功する
    let mv = p.select_move(&s).unwrap();
    assert_eq!(mv, Move::Place(Coord::new(2, 3)));
}

#[test]
fn external_engine_via_player_spec() {
    // PlayerSpec パーサ経由で生成 → Player として動作
    let path = mock_path("mock_engine_gtp.sh");
    let spec_str = format!("external:{},protocol=gtp,timeout=5", path.display());
    let spec = othello_player::parse_player_spec(&spec_str).expect("parse spec");
    let mut p = spec.build_player(Color::Black);
    assert_eq!(p.name(), "ExternalEnginePlayer(mock_engine_gtp.sh:gtp)");
    let s = GameState::standard_8x8();
    let mv = p.select_move(&s).expect("engine returned move");
    assert_eq!(mv, Move::Place(Coord::new(2, 3)));
}

#[test]
fn external_engine_handles_invalid_response() {
    // 存在しないバイナリを指定する → 起動失敗 → PlayerError
    let cfg = ExternalEngineConfig {
        command: PathBuf::from("/nonexistent/binary"),
        args: vec![],
        protocol: Protocol::Gtp,
        board_size: BoardSize::STANDARD,
        timeout: Duration::from_secs(2),
        working_dir: None,
        env: vec![],
    };
    let mut p = ExternalEnginePlayer::new(Color::Black, cfg);
    let s = GameState::standard_8x8();
    let r = p.select_move(&s);
    assert!(r.is_err());
}
