//! # othello-tui
//!
//! Othello シミュレータの TUI ( ratatui) 実装．以下のモードを提供する．
//!
//! - **Play** — 2 人対戦 ( Human vs Human)．カーソルを動かして石を置く．
//! - **Replay** — 棋譜の前後再生．`step_forward` / `step_backward` / `jump_to`．
//! - **Observe** — AI 同士の対戦を観戦する．
//!
//! ## ライブラリ API
//!
//! [`run_play`] / [`run_replay`] / [`run_observe`] は端末にアタッチしてイベントループを実行する．
//! 内部状態 [`AppState`] は描画ロジックから分離されており，テスト容易性を確保している．

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

/// Play モードを起動する ( Human vs Human)．
///
/// 端末に raw mode + alternate screen をセットアップし，イベントループを駆動する．
pub fn run_play(board_size: BoardSize) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_play_loop(&mut terminal, board_size);
    teardown_terminal(&mut terminal)?;
    result
}

/// Replay モードを起動する ( `GameHistory` を再生)．
pub fn run_replay(history: GameHistory) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_replay_loop(&mut terminal, history);
    teardown_terminal(&mut terminal)?;
    result
}

/// `GameRecord` から `GameHistory` を再構築する補助．
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
) -> Result<()> {
    let mut replay = ReplayMode::new(history);
    loop {
        let app = replay.snapshot();
        terminal.draw(|f| ui::render(f, &app))?;
        if event::poll(Duration::from_millis(200))? {
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
        } else if replay.auto_play {
            replay.handle(Action::StepForward);
        }
    }
    Ok(())
}

/// Observe モードの起動設定．
pub struct ObserveConfig {
    /// 盤面サイズ．
    pub board_size: BoardSize,
    /// 黒プレイヤー仕様．
    pub black_spec: PlayerSpec,
    /// 白プレイヤー仕様．
    pub white_spec: PlayerSpec,
    /// 乱数 seed ( 各プレイヤー SPEC の seed と XOR 合成される)．
    pub seed: u64,
    /// 自動再生間隔 ( ms)．`0` なら手動進行のみ．
    pub auto_delay_ms: u64,
}

/// Observe モードを起動する ( AI 対戦観戦)．
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
    let backend = ObserveBackend {
        black,
        white,
        names: (black_name, white_name),
    };
    let mut terminal = setup_terminal()?;
    let result = run_observe_loop(
        &mut terminal,
        config.board_size,
        backend,
        config.auto_delay_ms,
    );
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
