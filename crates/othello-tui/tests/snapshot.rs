//! TUI 描画結果のスナップショットテスト ( `insta` クレート使用)．
//!
//! `TestBackend` で描画した内容を文字列化し，`insta::assert_snapshot!` で固定する．
//! 8×8 標準初期配置と 1 手後の局面でカバレッジを取る．

use othello_core::{Coord, GameState, Move};
use othello_tui::{AppMode, AppState, Cursor, ui};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn render_to_string(app: &AppState, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| ui::render(f, app)).unwrap();
    let buffer = term.backend().buffer().clone();
    let mut s = String::new();
    for y in 0..buffer.area().height {
        for x in 0..buffer.area().width {
            let cell = &buffer[(x, y)];
            s.push_str(cell.symbol());
        }
        s.push('\n');
    }
    s
}

#[test]
fn snapshot_play_initial_8x8() {
    let mut app = AppState::new_play(GameState::standard_8x8());
    app.cursor = Cursor { row: 4, col: 4 };
    let s = render_to_string(&app, 80, 24);
    insta::assert_snapshot!("play_initial_8x8", s);
}

#[test]
fn snapshot_replay_after_one_move() {
    // 標準初期 → D3 ( 黒)．Replay モードで 1 手目時点を表示．
    let mut state = GameState::standard_8x8();
    state.apply_move(Move::Place(Coord::new(2, 3))).unwrap();
    let mut app = AppState::new_replay(state, 1);
    app.move_cursor = 1;
    app.mode = AppMode::Replay;
    let s = render_to_string(&app, 80, 24);
    insta::assert_snapshot!("replay_after_d3", s);
}
