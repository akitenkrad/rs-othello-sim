//! ratatui rendering logic. Renders an `AppState` into a `Frame`.
//!
//! The rendering logic is decoupled from state and verified by `TestBackend`
//! based snapshot tests.

use crate::app::{AppMode, AppState};
use othello_core::{Color, Coord};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders the entire app to the frame.
pub fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(10),   // body
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let mode = match app.mode {
        AppMode::Play => "Play",
        AppMode::Replay => "Replay",
        AppMode::Observe => "Observe",
    };
    let title = format!(
        "rs-othello-sim v{}    [Mode: {}]    [Move {}/{}]",
        env!("CARGO_PKG_VERSION"),
        mode,
        app.move_cursor,
        if app.total_moves == 0 {
            app.move_cursor
        } else {
            app.total_moves
        }
    );
    let block = Block::default().borders(Borders::ALL);
    let p = Paragraph::new(title).block(block);
    frame.render_widget(p, area);
}

fn render_body(frame: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    render_board(frame, chunks[0], app);
    render_info(frame, chunks[1], app);
}

fn render_board(frame: &mut Frame, area: Rect, app: &AppState) {
    let lines = build_board_lines(app);
    let block = Block::default().borders(Borders::ALL).title(" Board ");
    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

/// Convert the board into one `Line` per row. In Play mode the cursor cell
/// is wrapped in `[ ]`.
pub fn build_board_lines(app: &AppState) -> Vec<Line<'static>> {
    let size = app.state.board.size();
    let mut lines = Vec::with_capacity((size.rows as usize) + 2);

    // 列ヘッダ ( 各文字を 3 桁幅で配置．行内のセル幅 ` X ` と一致させる)
    let mut header = String::from("   ");
    for c in 0..size.cols {
        let ch = (b'a' + c) as char;
        header.push(' ');
        header.push(ch);
        header.push(' ');
    }
    lines.push(Line::from(header));

    for r in 0..size.rows {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::raw(format!("{:>2} ", r + 1)));
        for c in 0..size.cols {
            let glyph = match app.state.board.cell(Coord::new(r, c)) {
                Some(Color::Black) => 'X',
                Some(Color::White) => 'O',
                None => '.',
            };
            let is_cursor = app.mode == AppMode::Play && app.cursor.row == r && app.cursor.col == c;
            if is_cursor {
                spans.push(Span::styled(
                    format!("[{glyph}]"),
                    Style::default().add_modifier(Modifier::REVERSED),
                ));
            } else {
                spans.push(Span::raw(format!(" {glyph} ")));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn render_info(frame: &mut Frame, area: Rect, app: &AppState) {
    if app.mode == AppMode::Observe {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // game info
                Constraint::Min(3),    // move history
                Constraint::Length(4), // players
                Constraint::Length(8), // evaluator overlay
            ])
            .split(area);
        render_game_info(frame, chunks[0], app);
        render_history(frame, chunks[1], app);
        render_players(frame, chunks[2], app);
        render_evaluator(frame, chunks[3], app);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // game info
                Constraint::Min(3),    // move history
                Constraint::Length(4), // players
            ])
            .split(area);
        render_game_info(frame, chunks[0], app);
        render_history(frame, chunks[1], app);
        render_players(frame, chunks[2], app);
    }
}

fn render_evaluator(frame: &mut Frame, area: Rect, app: &AppState) {
    let block = Block::default().borders(Borders::ALL).title(" Evaluator ");
    let lines: Vec<Line<'static>> = match &app.evaluator {
        Some(overlay) => {
            let mut out: Vec<Line<'static>> = Vec::with_capacity(overlay.entries.len() + 1);
            out.push(Line::from(format!("Source: {}", overlay.source)));
            // 上位最大 5 行を表示
            let top = overlay.entries.iter().take(5);
            for entry in top {
                let bar_w = (entry.score.clamp(0.0, 1.0) * 8.0).round() as usize;
                let bar: String = std::iter::repeat_n('#', bar_w)
                    .chain(std::iter::repeat_n(' ', 8 - bar_w))
                    .collect();
                out.push(Line::from(format!(
                    "{:<4} [{}] {:.3}",
                    entry.move_label, bar, entry.score
                )));
            }
            out
        }
        None => vec![Line::from("Evaluator: N/A")],
    };
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_game_info(frame: &mut Frame, area: Rect, app: &AppState) {
    let black = app.state.board.count(Color::Black);
    let white = app.state.board.count(Color::White);
    let last = match app.state.last_move {
        Some(othello_core::Move::Place(c)) => format!("{}{}", (b'a' + c.col) as char, c.row + 1),
        Some(othello_core::Move::Pass) => "pass".to_string(),
        None => "-".to_string(),
    };
    let side_label = match app.state.side_to_move {
        Color::Black => "Black (X)",
        Color::White => "White (O)",
    };
    let lines = vec![
        Line::from(format!("Black (X): {black}   White (O): {white}")),
        Line::from(format!("Side: {side_label}")),
        Line::from(format!("Last: {last}")),
        Line::from(app.message.clone()),
    ];
    let block = Block::default().borders(Borders::ALL).title(" Game Info ");
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_history(frame: &mut Frame, area: Rect, app: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Move History ");
    let p = Paragraph::new(app.move_history.clone())
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_players(frame: &mut Frame, area: Rect, app: &AppState) {
    let lines = vec![
        Line::from(format!("Black (X): {}", app.players.0)),
        Line::from(format!("White (O): {}", app.players.1)),
    ];
    let block = Block::default().borders(Borders::ALL).title(" Players ");
    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let hint = match app.mode {
        AppMode::Play => "Keys: [hjkl/arrows] move  [Enter/Space] place  [p] pass  [q] quit",
        AppMode::Replay => {
            "Keys: [<-/->] step  [0/$] start/end  [Space/a] auto  [+/-] delay  [q] quit"
        }
        AppMode::Observe => "Keys: [Space] step  [a] auto-play  [+/-] delay  [q] quit",
    };
    let block = Block::default().borders(Borders::ALL);
    let p = Paragraph::new(hint).block(block);
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppMode, AppState, Cursor};
    use othello_core::GameState;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render_to_string(app: &AppState, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, app)).unwrap();
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
    fn renders_play_mode_initial() {
        let app = AppState::new_play(GameState::standard_8x8());
        let s = render_to_string(&app, 100, 30);
        assert!(s.contains("Mode: Play"));
        assert!(s.contains("Black"));
        // Each header letter sits in a 3-cell column ` X `; consecutive cells
        // share their separator space so the visible letters are 2 spaces apart.
        assert!(s.contains("a  b  c  d  e  f  g  h"));
    }

    #[test]
    fn renders_replay_mode_initial() {
        let mut app = AppState::new_replay(GameState::standard_8x8(), 5);
        app.cursor = Cursor { row: 0, col: 0 };
        let s = render_to_string(&app, 80, 24);
        assert!(s.contains("Mode: Replay"));
        assert!(s.contains("Move 0/5"));
    }

    #[test]
    fn play_cursor_is_visible_in_buffer() {
        // Cursor を空マスに置いた場合 `[.]` が含まれるはず．
        let mut app = AppState::new_play(GameState::standard_8x8());
        app.mode = AppMode::Play;
        app.cursor = Cursor { row: 0, col: 0 };
        let s = render_to_string(&app, 80, 24);
        assert!(s.contains("[.]"));
    }
}
