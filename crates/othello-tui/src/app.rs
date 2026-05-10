//! TUI application state. Decoupled from rendering and input logic to make
//! it independently testable.

use othello_core::{Color, Coord, GameState, Move};

/// UI mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Play (two-player match).
    Play,
    /// Replay (replay a game record).
    Replay,
    /// Observe (watch an AI vs AI match).
    Observe,
}

/// Cursor position on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Row (0-based).
    pub row: u8,
    /// Column (0-based).
    pub col: u8,
}

impl Cursor {
    /// Returns a position near the center of the board.
    #[must_use]
    pub fn center(state: &GameState) -> Self {
        let size = state.board.size();
        Self {
            row: size.rows / 2,
            col: size.cols / 2,
        }
    }

    /// Returns the cursor as a `Coord`.
    #[must_use]
    pub fn as_coord(&self) -> Coord {
        Coord::new(self.row, self.col)
    }

    /// Moves the cursor up/down/left/right, clamped to the board.
    pub fn move_by(&mut self, drow: i8, dcol: i8, state: &GameState) {
        let size = state.board.size();
        let nr = (self.row as i16 + drow as i16).clamp(0, size.rows as i16 - 1);
        let nc = (self.col as i16 + dcol as i16).clamp(0, size.cols as i16 - 1);
        self.row = nr as u8;
        self.col = nc as u8;
    }
}

/// One row in the evaluator overlay: `(move label, score, display order)`.
///
/// Stored in `AppState` already sorted by score in descending order.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorEntry {
    /// String representation of the move (e.g. `"D3"`, `"pass"`).
    pub move_label: String,
    /// Score in `[0.0, 1.0]`.
    pub score: f32,
}

/// State for the evaluator overlay.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvaluatorOverlay {
    /// Player name shown in the header.
    pub source: String,
    /// Rows sorted by score in descending order.
    pub entries: Vec<EvaluatorEntry>,
}

/// Visible-state snapshot of the TUI app, consumed by [`crate::ui::render`].
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current UI mode.
    pub mode: AppMode,
    /// Game state being displayed.
    pub state: GameState,
    /// Cursor position on the board (only meaningful in Play mode).
    pub cursor: Cursor,
    /// Side-to-move / status message (e.g. "Black to move", "Pass").
    pub message: String,
    /// Player names (black, white).
    pub players: (String, String),
    /// String representation of the move sequence (e.g. "1. d3 c5  2. e3 ...").
    pub move_history: String,
    /// Index of the current move being shown (a **move-count cursor**, not
    /// the board `cursor`).
    pub move_cursor: usize,
    /// Total number of moves (used by Replay mode to render `<n>/<total>`).
    pub total_moves: usize,
    /// Whether auto-play is active (Replay mode).
    pub auto_play: bool,
    /// Whether the game has ended.
    pub finished: bool,
    /// Evaluator overlay (e.g. MCTS visit counts in Observe mode).
    pub evaluator: Option<EvaluatorOverlay>,
}

impl AppState {
    /// Builds the initial snapshot for Play mode.
    #[must_use]
    pub fn new_play(state: GameState) -> Self {
        let cursor = Cursor::center(&state);
        let message = format!("{:?} to move", state.side_to_move);
        Self {
            mode: AppMode::Play,
            state,
            cursor,
            message,
            players: ("Human".into(), "Human".into()),
            move_history: String::new(),
            move_cursor: 0,
            total_moves: 0,
            auto_play: false,
            finished: false,
            evaluator: None,
        }
    }

    /// Builds the initial snapshot for Observe mode.
    #[must_use]
    pub fn new_observe(state: GameState, players: (String, String)) -> Self {
        let cursor = Cursor::center(&state);
        Self {
            mode: AppMode::Observe,
            state,
            cursor,
            message: String::from("Observe (Space=step, a=auto, +/-=delay, q=quit)"),
            players,
            move_history: String::new(),
            move_cursor: 0,
            total_moves: 0,
            auto_play: false,
            finished: false,
            evaluator: None,
        }
    }

    /// Builds the initial snapshot for Replay mode.
    #[must_use]
    pub fn new_replay(state: GameState, total: usize) -> Self {
        let cursor = Cursor::center(&state);
        Self {
            mode: AppMode::Replay,
            state,
            cursor,
            message: format!("Move 0/{total}"),
            players: ("?".into(), "?".into()),
            move_history: String::new(),
            move_cursor: 0,
            total_moves: total,
            auto_play: false,
            finished: false,
            evaluator: None,
        }
    }
}

/// Helper that formats the most recent moves into a human-readable string.
#[must_use]
pub fn format_move_history(moves: &[(Color, Move)], up_to: usize) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let take = up_to.min(moves.len());
    for (idx, (_side, mv)) in moves.iter().take(take).enumerate() {
        if idx % 2 == 0 {
            let _ = write!(s, "{}. ", (idx / 2) + 1);
        }
        let token = match mv {
            Move::Place(c) => format!("{}{}", (b'a' + c.col) as char, c.row + 1),
            Move::Pass => "pass".to_string(),
        };
        let _ = write!(s, "{token}");
        s.push(' ');
    }
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_center_8x8() {
        let s = GameState::standard_8x8();
        let c = Cursor::center(&s);
        assert_eq!(c.row, 4);
        assert_eq!(c.col, 4);
    }

    #[test]
    fn cursor_move_clamps() {
        let s = GameState::standard_8x8();
        let mut c = Cursor { row: 0, col: 0 };
        c.move_by(-1, -1, &s);
        assert_eq!(c, Cursor { row: 0, col: 0 });
        c.move_by(100, 100, &s);
        assert_eq!(c, Cursor { row: 7, col: 7 });
    }

    #[test]
    fn format_history_basic() {
        let moves = vec![
            (Color::Black, Move::Place(Coord::new(2, 3))),
            (Color::White, Move::Place(Coord::new(2, 2))),
            (Color::Black, Move::Place(Coord::new(2, 4))),
        ];
        let s = format_move_history(&moves, 3);
        assert_eq!(s, "1. d3 c3 2. e3");
    }
}
