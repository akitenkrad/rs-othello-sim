//! TUI アプリケーション状態．描画と入力ロジックから独立してテスト可能にする．

use othello_core::{Color, Coord, GameState, Move};

/// UI モード．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Play ( 2 人対戦)．
    Play,
    /// Replay ( 棋譜再生)．
    Replay,
    /// Observe ( AI 対戦観戦)．
    Observe,
}

/// 盤面上のカーソル位置．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// 行 ( 0 起点)．
    pub row: u8,
    /// 列 ( 0 起点)．
    pub col: u8,
}

impl Cursor {
    /// 中央付近の位置を返す．
    #[must_use]
    pub fn center(state: &GameState) -> Self {
        let size = state.board.size();
        Self {
            row: size.rows / 2,
            col: size.cols / 2,
        }
    }

    /// `Coord` 表現．
    #[must_use]
    pub fn as_coord(&self) -> Coord {
        Coord::new(self.row, self.col)
    }

    /// 上下左右に移動する ( 範囲クランプ)．
    pub fn move_by(&mut self, drow: i8, dcol: i8, state: &GameState) {
        let size = state.board.size();
        let nr = (self.row as i16 + drow as i16).clamp(0, size.rows as i16 - 1);
        let nc = (self.col as i16 + dcol as i16).clamp(0, size.cols as i16 - 1);
        self.row = nr as u8;
        self.col = nc as u8;
    }
}

/// Evaluator overlay 1 行: `(move 表記, 評価値, 表示順)`．
///
/// 大きい順にソートされた状態で `AppState` に格納される．
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatorEntry {
    /// 手の文字列表現 ( 例 `"D3"`，`"pass"`)．
    pub move_label: String,
    /// 評価値 ( 0.0 〜 1.0)．
    pub score: f32,
}

/// Evaluator overlay の状態．
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvaluatorOverlay {
    /// プレイヤー名 ( 表示ヘッダ用)．
    pub source: String,
    /// 行リスト ( score 降順)．
    pub entries: Vec<EvaluatorEntry>,
}

/// TUI アプリの可視状態スナップショット．描画関数 [`crate::ui::render`] が読む．
#[derive(Debug, Clone)]
pub struct AppState {
    /// 現在の UI モード．
    pub mode: AppMode,
    /// 表示中のゲーム状態．
    pub state: GameState,
    /// 盤面上のカーソル位置 ( Play モードでのみ意味がある)．
    pub cursor: Cursor,
    /// 手番情報メッセージ ( 「Black to move」「Pass」等)．
    pub message: String,
    /// プレイヤー名 ( 黒/白)．
    pub players: (String, String),
    /// 着手列の文字列表現 ( "1. d3 c5  2. e3 ...")．
    pub move_history: String,
    /// 現在何手目を見ているか (`cursor` ではなく**手数のカーソル**)．
    pub move_cursor: usize,
    /// 総手数 ( Replay モードで `<n>/<total>` 表示用)．
    pub total_moves: usize,
    /// 自動再生中フラグ ( Replay モード)．
    pub auto_play: bool,
    /// 終局かどうか．
    pub finished: bool,
    /// Evaluator overlay ( Observe モードで MCTS 等の visit count を表示)．
    pub evaluator: Option<EvaluatorOverlay>,
}

impl AppState {
    /// Play モード用の初期スナップショットを作る．
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

    /// Observe モード用の初期スナップショットを作る．
    #[must_use]
    pub fn new_observe(state: GameState, players: (String, String)) -> Self {
        let cursor = Cursor::center(&state);
        Self {
            mode: AppMode::Observe,
            state,
            cursor,
            message: String::from("Observe ( Space=step, a=auto, +/-=delay, q=quit)"),
            players,
            move_history: String::new(),
            move_cursor: 0,
            total_moves: 0,
            auto_play: false,
            finished: false,
            evaluator: None,
        }
    }

    /// Replay モード用の初期スナップショットを作る．
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

/// 直近何手分かをヒューマン可読な形式で文字列化する補助．
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
