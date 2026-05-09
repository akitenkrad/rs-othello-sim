//! # othello-tui
//!
//! Othello シミュレータの TUI ( ratatui) 実装．以下のモードを提供する．
//!
//! - **Play** — 2 人対戦 ( Human vs Human)．カーソルを動かして石を置く．
//! - **Replay** — 棋譜の前後再生．`step_forward` / `step_backward` / `jump_to`．
//!
//! `Observe` モード ( AI 対戦観戦) は Phase 4 で実装予定．
//!
//! ## ライブラリ API
//!
//! [`run_play`] / [`run_replay`] は端末にアタッチしてイベントループを実行する．
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
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{Stdout, stdout};
use std::time::Duration;

pub use app::{AppMode, AppState, Cursor};
pub use input::Action;
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
