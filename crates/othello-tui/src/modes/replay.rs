//! Replay モード ( 棋譜再生) のロジック．

use crate::app::{AppMode, AppState, Cursor, format_move_history};
use crate::input::Action;
use othello_core::{Color, Move};
use othello_engine::GameHistory;

/// Replay モード状態．
#[derive(Debug)]
pub struct ReplayMode {
    history: GameHistory,
    cursor: usize,
    /// 自動再生中ならば `true`．`Action::ToggleAutoPlay` で切り替わる．
    pub auto_play: bool,
    moves: Vec<(Color, Move)>,
    players: (String, String),
}

impl ReplayMode {
    /// 履歴を消費して Replay モードを開始する．
    #[must_use]
    pub fn new(history: GameHistory) -> Self {
        let moves = build_move_log(&history);
        Self {
            history,
            cursor: 0,
            auto_play: false,
            moves,
            players: ("Black".into(), "White".into()),
        }
    }

    /// プレイヤー名を設定する ( 表示用)．
    pub fn set_players(&mut self, black: impl Into<String>, white: impl Into<String>) {
        self.players = (black.into(), white.into());
    }

    /// 1 アクションを処理する．
    pub fn handle(&mut self, action: Action) {
        match action {
            Action::StepForward => {
                if self.cursor < self.history.total_moves() {
                    self.cursor += 1;
                }
            }
            Action::StepBackward => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            Action::JumpStart => {
                self.cursor = 0;
            }
            Action::JumpEnd => {
                self.cursor = self.history.total_moves();
            }
            Action::ToggleAutoPlay => {
                self.auto_play = !self.auto_play;
            }
            _ => {}
        }
    }

    /// 任意手数にジャンプする ( 範囲外は無視)．
    pub fn jump_to(&mut self, n: usize) {
        if n <= self.history.total_moves() {
            self.cursor = n;
        }
    }

    /// 描画用スナップショットを生成する．
    #[must_use]
    pub fn snapshot(&self) -> AppState {
        let state = self
            .history
            .snapshot_at(self.cursor)
            .expect("cursor in range")
            .clone();
        let cursor = Cursor::center(&state);
        let total = self.history.total_moves();
        let message = format!(
            "Move {}/{}{}",
            self.cursor,
            total,
            if self.auto_play { " [AUTO]" } else { "" }
        );
        AppState {
            mode: AppMode::Replay,
            state,
            cursor,
            message,
            players: self.players.clone(),
            move_history: format_move_history(&self.moves, self.cursor),
            move_cursor: self.cursor,
            total_moves: total,
            auto_play: self.auto_play,
            finished: self.cursor == total,
            evaluator: None,
        }
    }

    /// テスト用: カーソル位置を返す．
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

fn build_move_log(history: &GameHistory) -> Vec<(Color, Move)> {
    // history.moves の i 番目を打ったのは snapshots[i].side_to_move
    let mut log = Vec::with_capacity(history.total_moves());
    for (i, mv) in history.moves().iter().enumerate() {
        let pre = history.snapshot_at(i).expect("snapshot in range");
        log.push((pre.side_to_move, *mv));
    }
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::{Coord, GameState, Move};

    fn build_history() -> GameHistory {
        // 標準初期局面 → D3 ( 黒) → C5 ( 白) → D6 ( 黒) という典型的開始 3 手．
        let mut s = GameState::standard_8x8();
        let mut h = GameHistory::new(s.clone());
        for mv in [
            Move::Place(Coord::new(2, 3)),
            Move::Place(Coord::new(4, 2)),
            Move::Place(Coord::new(5, 3)),
        ] {
            s.apply_move(mv).unwrap();
            h.push(mv, s.clone());
        }
        h
    }

    #[test]
    fn step_forward_and_backward() {
        let mut r = ReplayMode::new(build_history());
        assert_eq!(r.cursor(), 0);
        r.handle(Action::StepForward);
        assert_eq!(r.cursor(), 1);
        r.handle(Action::StepBackward);
        assert_eq!(r.cursor(), 0);
    }

    #[test]
    fn jump_start_and_end() {
        let mut r = ReplayMode::new(build_history());
        r.handle(Action::JumpEnd);
        assert_eq!(r.cursor(), 3);
        r.handle(Action::JumpStart);
        assert_eq!(r.cursor(), 0);
    }

    #[test]
    fn toggle_auto_play() {
        let mut r = ReplayMode::new(build_history());
        assert!(!r.auto_play);
        r.handle(Action::ToggleAutoPlay);
        assert!(r.auto_play);
        r.handle(Action::ToggleAutoPlay);
        assert!(!r.auto_play);
    }

    #[test]
    fn snapshot_total_matches_history() {
        let r = ReplayMode::new(build_history());
        assert_eq!(r.snapshot().total_moves, 3);
    }

    #[test]
    fn jump_to_arbitrary() {
        let mut r = ReplayMode::new(build_history());
        r.jump_to(2);
        assert_eq!(r.cursor(), 2);
        r.jump_to(99); // ignored
        assert_eq!(r.cursor(), 2);
    }
}
