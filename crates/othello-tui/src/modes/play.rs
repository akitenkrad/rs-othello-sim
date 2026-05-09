//! Play モード ( Human vs Human) のロジック．

use crate::app::{AppState, Cursor, format_move_history};
use crate::input::Action;
use othello_core::{BoardSize, Color, GameState, Move, OthelloError};

/// Play モード状態．
#[derive(Debug)]
pub struct PlayMode {
    state: GameState,
    cursor: Cursor,
    moves: Vec<(Color, Move)>,
    message: String,
    finished: bool,
}

impl PlayMode {
    /// 標準初期局面で Play モードを開始する．
    pub fn new(board_size: BoardSize) -> Result<Self, OthelloError> {
        let state = GameState::standard(board_size)?;
        let cursor = Cursor::center(&state);
        let message = format!("{:?} to move", state.side_to_move);
        Ok(Self {
            state,
            cursor,
            moves: Vec::new(),
            message,
            finished: false,
        })
    }

    /// アクションを 1 つ処理する．
    pub fn handle(&mut self, action: Action) {
        if self.finished {
            return;
        }
        // 直前メッセージがエラー類のときは上書きしないようにフラグで管理する．
        let mut keep_message = false;
        match action {
            Action::Up => self.cursor.move_by(-1, 0, &self.state),
            Action::Down => self.cursor.move_by(1, 0, &self.state),
            Action::Left => self.cursor.move_by(0, -1, &self.state),
            Action::Right => self.cursor.move_by(0, 1, &self.state),
            Action::Place => {
                let advanced = self.try_place();
                keep_message = !advanced;
            }
            Action::Pass => {
                let advanced = self.try_pass();
                keep_message = !advanced;
            }
            _ => {}
        }
        if self.state.is_terminal() {
            self.finished = true;
            if let Some(r) = self.state.result() {
                self.message = format!(
                    "Game over: black={} white={} winner={:?}",
                    r.black, r.white, r.winner
                );
            }
        } else if !keep_message {
            self.message = format!("{:?} to move", self.state.side_to_move);
        }
    }

    /// 着手を試みる．成功時 `true`，失敗時 `false`．
    fn try_place(&mut self) -> bool {
        let coord = self.cursor.as_coord();
        let mv = Move::Place(coord);
        let side = self.state.side_to_move;
        match self.state.apply_move(mv) {
            Ok(_) => {
                self.moves.push((side, mv));
                true
            }
            Err(e) => {
                self.message = format!("Illegal: {e}");
                false
            }
        }
    }

    /// パスを試みる．成功時 `true`，失敗時 `false`．
    fn try_pass(&mut self) -> bool {
        let side = self.state.side_to_move;
        if !self.state.legal_moves().is_empty() {
            self.message = "Pass not allowed: you have legal moves".into();
            return false;
        }
        match self.state.apply_move(Move::Pass) {
            Ok(_) => {
                self.moves.push((side, Move::Pass));
                true
            }
            Err(e) => {
                self.message = format!("Pass rejected: {e}");
                false
            }
        }
    }

    /// 終局しているか．
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// 描画用スナップショットを生成する．
    #[must_use]
    pub fn snapshot(&self) -> AppState {
        let total = self.moves.len();
        AppState {
            mode: crate::app::AppMode::Play,
            state: self.state.clone(),
            cursor: self.cursor,
            message: self.message.clone(),
            players: ("Human".into(), "Human".into()),
            move_history: format_move_history(&self.moves, total),
            move_cursor: total,
            total_moves: total,
            auto_play: false,
            finished: self.finished,
            evaluator: None,
        }
    }

    /// テスト用: 現在の `GameState` を借用する．
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// テスト用: カーソル参照．
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::Coord;

    #[test]
    fn cursor_movement() {
        let mut p = PlayMode::new(BoardSize::STANDARD).unwrap();
        let initial = p.cursor();
        p.handle(Action::Up);
        assert_eq!(p.cursor().row, initial.row - 1);
        p.handle(Action::Right);
        assert_eq!(p.cursor().col, initial.col + 1);
    }

    #[test]
    fn place_legal_move_advances_state() {
        let mut p = PlayMode::new(BoardSize::STANDARD).unwrap();
        // 標準初期局面で D3 ( row=2, col=3) は黒の合法手
        p.handle(Action::Up); // 4,4 → 3,4
        p.handle(Action::Up); // 3,4 → 2,4
        p.handle(Action::Left); // 2,4 → 2,3
        p.handle(Action::Place);
        assert_eq!(p.state().side_to_move, Color::White);
        assert_eq!(p.state().move_number, 1);
    }

    #[test]
    fn place_illegal_move_keeps_state() {
        let mut p = PlayMode::new(BoardSize::STANDARD).unwrap();
        // 中央 (4,4) は石があるので置けない
        p.handle(Action::Place);
        assert_eq!(p.state().move_number, 0);
    }

    #[test]
    fn pass_rejected_when_legal_moves_exist() {
        let mut p = PlayMode::new(BoardSize::STANDARD).unwrap();
        p.handle(Action::Pass);
        assert_eq!(p.state().move_number, 0);
        assert!(p.snapshot().message.contains("Pass not allowed"));
    }

    #[test]
    fn snapshot_reflects_state() {
        let mut p = PlayMode::new(BoardSize::STANDARD).unwrap();
        p.handle(Action::Up);
        p.handle(Action::Up);
        p.handle(Action::Left);
        p.handle(Action::Place);
        let snap = p.snapshot();
        assert_eq!(snap.total_moves, 1);
        assert!(snap.move_history.contains("d3"));
        assert_eq!(snap.cursor.as_coord(), Coord::new(2, 3));
    }
}
