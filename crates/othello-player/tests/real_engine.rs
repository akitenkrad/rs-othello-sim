//! Real-engine smoke tests for `ExternalEnginePlayer`.
//!
//! Phase 6.1: these tests exercise actual Edax / Egaroucid binaries placed
//! under `vendor/engines/`. They are gated behind `#[ignore]` so they never
//! run on `cargo test` (CI does not build engines). Run them locally with:
//!
//! ```bash
//! bash scripts/fetch_engines.sh all
//! cargo test -p othello-player --test real_engine -- --ignored
//! ```
//!
//! When a binary is missing, each test prints a `[skip]` notice and returns
//! successfully so users who installed only one of the two engines can still
//! exercise the available smoke tests with `-- --ignored`.

#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use othello_core::prelude::*;
use othello_player::external::{ExternalEngineConfig, ExternalEnginePlayer, Protocol};
use othello_player::traits::Player;

/// Repository root (`<crate>/../..`).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has parent")
        .parent()
        .expect("crates/ dir has parent")
        .to_path_buf()
}

/// Search for an Edax binary in known locations.
fn edax_path() -> Option<PathBuf> {
    let candidates = [
        "vendor/engines/edax/bin/lEdax-x64-modern",
        "vendor/engines/edax/bin/lEdax-arm",
        "vendor/engines/edax/bin/mEdax",
        "vendor/engines/edax/bin/edax",
    ];
    for c in candidates {
        let p = repo_root().join(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Search for an Egaroucid binary in known locations.
fn egaroucid_path() -> Option<PathBuf> {
    let candidates = [
        "vendor/engines/egaroucid/bin/Egaroucid_for_Console",
        "vendor/engines/egaroucid/Egaroucid_for_Console",
        "vendor/engines/egaroucid/bin/egaroucid",
        "vendor/engines/egaroucid/egaroucid",
    ];
    for c in candidates {
        let p = repo_root().join(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Build an `ExternalEnginePlayer` for the given binary and protocol.
fn build_engine(path: PathBuf, protocol: Protocol, color: Color) -> ExternalEnginePlayer {
    let config = ExternalEngineConfig {
        command: path,
        args: vec![],
        protocol,
        board_size: BoardSize::STANDARD,
        timeout: Duration::from_secs(30),
        working_dir: None,
        env: vec![],
    };
    ExternalEnginePlayer::new(color, config)
}

#[test]
#[ignore = "requires vendor/engines/edax (run scripts/fetch_engines.sh edax)"]
fn edax_returns_a_legal_move() {
    let Some(path) = edax_path() else {
        eprintln!("[skip] edax binary not installed under vendor/engines/edax/");
        return;
    };

    let mut player = build_engine(path, Protocol::Ntest, Color::Black);
    let state = GameState::standard_8x8();
    let mv = player
        .select_move(&state)
        .expect("edax should return a legal move on the opening position");

    let legal = state.board.legal_moves(Color::Black);
    assert!(
        legal.contains(&mv),
        "edax returned a non-legal move: {mv:?}; legal moves were {legal:?}"
    );
}

#[test]
#[ignore = "requires vendor/engines/egaroucid (run scripts/fetch_engines.sh egaroucid)"]
fn egaroucid_returns_a_legal_move() {
    let Some(path) = egaroucid_path() else {
        eprintln!("[skip] egaroucid binary not installed under vendor/engines/egaroucid/");
        return;
    };

    let mut player = build_engine(path, Protocol::Gtp, Color::Black);
    let state = GameState::standard_8x8();
    let mv = player
        .select_move(&state)
        .expect("egaroucid should return a legal move on the opening position");

    let legal = state.board.legal_moves(Color::Black);
    assert!(
        legal.contains(&mv),
        "egaroucid returned a non-legal move: {mv:?}; legal moves were {legal:?}"
    );
}

#[test]
#[ignore = "requires both vendor/engines/edax and vendor/engines/egaroucid"]
fn edax_vs_egaroucid_short_match() {
    let Some(edax) = edax_path() else {
        eprintln!("[skip] edax binary not installed");
        return;
    };
    let Some(egaroucid) = egaroucid_path() else {
        eprintln!("[skip] egaroucid binary not installed");
        return;
    };

    let mut black = build_engine(edax, Protocol::Ntest, Color::Black);
    let mut white = build_engine(egaroucid, Protocol::Gtp, Color::White);

    // Avoid pulling in `othello-engine` as a dev-dependency (would create a
    // circular dependency since othello-engine -> othello-player). Run a
    // small hand-rolled loop instead, capped at a few plies.
    let mut state = GameState::standard_8x8();
    const MAX_PLIES: u32 = 6;

    while !state.is_terminal() && state.move_number < MAX_PLIES {
        let legal = state.legal_moves();
        if legal.is_empty() {
            // forced pass
            state
                .apply_move(Move::Pass)
                .expect("pass is legal when no other move exists");
            continue;
        }
        let player: &mut dyn Player = if state.side_to_move == Color::Black {
            &mut black
        } else {
            &mut white
        };
        let mv = player
            .select_move(&state)
            .expect("real engine should respond within timeout");
        assert!(
            legal.contains(&mv),
            "engine returned a non-legal move at ply {}: {mv:?}; legal={legal:?}",
            state.move_number
        );
        state
            .apply_move(mv)
            .expect("apply_move accepts engine output");
    }
}
