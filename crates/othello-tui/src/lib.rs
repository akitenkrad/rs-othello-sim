//! # othello-tui
//!
//! TUI (ratatui) frontend for the Othello simulator. The crate provides three
//! modes:
//!
//! - **Play** — two-player match (Human vs Human). Move the cursor to place a stone.
//! - **Replay** — step a game record forward and backward via `step_forward` /
//!   `step_backward` / `jump_to`.
//! - **Observe** — watch an AI vs AI match.
//!
//! ## Library API
//!
//! [`run_play`] / [`run_replay`] / [`run_observe`] attach to the terminal and
//! drive the event loop. Internal state ([`AppState`]) is decoupled from the
//! render logic to keep the code easy to test.

pub mod app;
pub mod input;
pub mod modes;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use othello_core::BoardSize;
use othello_engine::GameHistory;
use othello_player::PlayerSpec;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{Stdout, stdout};
use std::time::{Duration, Instant};

pub use app::{AppMode, AppState, Cursor, EvaluatorEntry, EvaluatorOverlay};
pub use input::Action;
pub use modes::observe::{ObserveBackend, ObserveMode};
pub use modes::play::PlayMode;
pub use modes::replay::ReplayMode;

/// Launch Play mode (Human vs Human).
///
/// Enables raw mode plus the alternate screen on the terminal and drives the
/// event loop.
pub fn run_play(board_size: BoardSize) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_play_loop(&mut terminal, board_size);
    teardown_terminal(&mut terminal)?;
    result
}

/// Launch Replay mode (replay a `GameHistory`).
///
/// By default the user advances manually. To start with auto-play enabled,
/// use [`run_replay_with_options`].
pub fn run_replay(history: GameHistory) -> Result<()> {
    run_replay_with_options(history, ReplayOptions::default())
}

/// Options for launching Replay mode.
#[derive(Debug, Clone, Copy)]
pub struct ReplayOptions {
    /// Whether to start auto-play immediately on launch.
    pub auto_play: bool,
    /// Auto-play step interval in milliseconds. Clamped to `[50, 5000]`.
    pub auto_delay_ms: u64,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            auto_play: false,
            auto_delay_ms: modes::replay::DEFAULT_AUTO_DELAY_MS,
        }
    }
}

/// Launch Replay mode with explicit options.
pub fn run_replay_with_options(history: GameHistory, options: ReplayOptions) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_replay_loop(&mut terminal, history, options);
    teardown_terminal(&mut terminal)?;
    result
}

/// Helper that reconstructs a `GameHistory` from a `GameRecord`.
pub fn record_to_history(record: &othello_io::GameRecord) -> Result<GameHistory> {
    use othello_core::GameState;
    let size = record.metadata.board_size;
    let mut state = GameState::standard(size)
        .map_err(|e| anyhow::anyhow!("failed to build standard state: {e}"))?;
    let mut history = GameHistory::new(state.clone());
    for entry in &record.moves {
        state
            .apply_move(entry.r#move)
            .map_err(|e| anyhow::anyhow!("failed to apply move {:?}: {e}", entry.r#move))?;
        history.push(entry.r#move, state.clone());
    }
    Ok(history)
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_play_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    board_size: BoardSize,
) -> Result<()> {
    let mut play =
        PlayMode::new(board_size).map_err(|e| anyhow::anyhow!("invalid board size: {e}"))?;
    loop {
        let app = play.snapshot();
        terminal.draw(|f| ui::render(f, &app))?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match input::map_play_key(key.code) {
                    Some(Action::Quit) => break,
                    Some(action) => {
                        play.handle(action);
                    }
                    None => {}
                }
                if play.is_finished() {
                    // 終局後は最終状態を描画してから q 待ち．
                    let app = play.snapshot();
                    terminal.draw(|f| ui::render(f, &app))?;
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn run_replay_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    history: GameHistory,
    options: ReplayOptions,
) -> Result<()> {
    let mut replay = ReplayMode::with_options(history, options.auto_play, options.auto_delay_ms);
    let mut last_step = Instant::now();
    loop {
        let app = replay.snapshot();
        terminal.draw(|f| ui::render(f, &app))?;
        // auto_play 中は一定ミリ秒ごとに poll を切り上げて手を進める
        let timeout = if replay.auto_play {
            Duration::from_millis(replay.auto_delay_ms.min(50))
        } else {
            Duration::from_millis(200)
        };
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match input::map_replay_key(key.code) {
                    Some(Action::Quit) => break,
                    Some(action) => replay.handle(action),
                    None => {}
                }
            }
        } else if replay.auto_play
            && !replay.is_finished()
            && last_step.elapsed() >= Duration::from_millis(replay.auto_delay_ms)
        {
            replay.auto_advance();
            last_step = Instant::now();
        }
    }
    Ok(())
}

/// Configuration for launching Observe mode.
pub struct ObserveConfig {
    /// Board size.
    pub board_size: BoardSize,
    /// Black player spec.
    pub black_spec: PlayerSpec,
    /// White player spec.
    pub white_spec: PlayerSpec,
    /// Random seed (XOR-mixed with the seed in each player SPEC).
    pub seed: u64,
    /// Auto-play interval in milliseconds. `0` means manual stepping only.
    pub auto_delay_ms: u64,
}

/// Launch Observe mode (watch an AI vs AI match).
///
/// The `Nn` variant cannot be built from this crate (to avoid a Candle
/// dependency). When the SPEC contains `Nn`, build the Player from
/// `othello-cli` first and call [`run_observe_with_players`].
pub fn run_observe(config: ObserveConfig) -> Result<()> {
    use othello_core::Color;
    use othello_player::player_spec;

    // SPEC の seed を XOR 合成して factory 風に Player を生成
    fn build(spec: PlayerSpec, color: Color, seed: u64) -> Box<dyn othello_player::Player> {
        let overridden = match spec {
            PlayerSpec::Random { seed: s } => PlayerSpec::Random { seed: s ^ seed },
            PlayerSpec::Greedy => PlayerSpec::Greedy,
            PlayerSpec::Mcts {
                simulations,
                exploration,
                seed: s,
                max_rollout_depth,
            } => PlayerSpec::Mcts {
                simulations,
                exploration,
                seed: Some(s.unwrap_or(0) ^ seed),
                max_rollout_depth,
            },
            external @ PlayerSpec::External { .. } => external,
            PlayerSpec::Nn(_) => panic!(
                "PlayerSpec::Nn must be constructed via othello-cli; \
                 use run_observe_with_players from the CLI wrapper"
            ),
        };
        overridden.build_player(color)
    }

    let black_name = player_spec::spec_name(&config.black_spec).to_string();
    let white_name = player_spec::spec_name(&config.white_spec).to_string();
    let black = build(config.black_spec.clone(), Color::Black, config.seed);
    let white = build(
        config.white_spec.clone(),
        Color::White,
        config.seed.wrapping_add(0x9E37_79B9),
    );
    run_observe_with_players(
        config.board_size,
        black,
        white,
        black_name,
        white_name,
        config.auto_delay_ms,
    )
}

/// Launch Observe mode with already-built `Box<dyn Player>` instances.
///
/// This entry point exists so `othello-cli` can handle the `Nn` variant.
pub fn run_observe_with_players(
    board_size: BoardSize,
    black: Box<dyn othello_player::Player>,
    white: Box<dyn othello_player::Player>,
    black_name: String,
    white_name: String,
    auto_delay_ms: u64,
) -> Result<()> {
    let backend = ObserveBackend {
        black,
        white,
        names: (black_name, white_name),
    };
    let mut terminal = setup_terminal()?;
    let result = run_observe_loop(&mut terminal, board_size, backend, auto_delay_ms);
    teardown_terminal(&mut terminal)?;
    result
}

fn run_observe_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    board_size: BoardSize,
    backend: ObserveBackend,
    auto_delay_ms: u64,
) -> Result<()> {
    let mut observe = ObserveMode::new(board_size, backend, auto_delay_ms)
        .map_err(|e| anyhow::anyhow!("invalid board size: {e}"))?;
    let mut last_step = Instant::now();
    loop {
        let app = observe.snapshot();
        terminal.draw(|f| ui::render(f, &app))?;
        let timeout = if observe.auto_play && observe.auto_delay_ms > 0 {
            // auto-delay まで待機
            Duration::from_millis(observe.auto_delay_ms.min(50))
        } else {
            Duration::from_millis(200)
        };
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match input::map_observe_key(key.code) {
                    Some(Action::Quit) => break,
                    Some(action) => observe.handle(action),
                    None => {}
                }
            }
        } else if observe.auto_play
            && observe.auto_delay_ms > 0
            && !observe.is_finished()
            && last_step.elapsed() >= Duration::from_millis(observe.auto_delay_ms)
        {
            observe.advance_one();
            last_step = Instant::now();
        }
        if observe.is_finished() && !observe.auto_play {
            // 終局後はキー入力のみ受け付ける ( quit 待ち)．特に処理なし．
        }
    }
    Ok(())
}
