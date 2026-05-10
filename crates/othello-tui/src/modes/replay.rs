//! Replay モード ( 棋譜再生) のロジック．

use crate::app::{AppMode, AppState, Cursor, format_move_history};
use crate::input::Action;
use othello_core::{Color, Move};
use othello_engine::GameHistory;

/// 自動再生間隔の下限・上限・刻み ( ミリ秒)．
const AUTO_DELAY_MIN_MS: u64 = 50;
const AUTO_DELAY_MAX_MS: u64 = 5_000;
const AUTO_DELAY_STEP_MS: u64 = 100;

/// 自動再生間隔のデフォルト ( ミリ秒)．500 ms = 2 手/秒．
pub const DEFAULT_AUTO_DELAY_MS: u64 = 500;

fn clamp_delay(ms: u64) -> u64 {
    ms.clamp(AUTO_DELAY_MIN_MS, AUTO_DELAY_MAX_MS)
}

/// Replay モード状態．
#[derive(Debug)]
pub struct ReplayMode {
    history: GameHistory,
    cursor: usize,
    /// 自動再生中ならば `true`．`Action::ToggleAutoPlay` で切り替わる．
    pub auto_play: bool,
    /// 自動再生時の手間隔 ( ミリ秒)．`Action::IncreaseDelay` / `DecreaseDelay` で増減．
    pub auto_delay_ms: u64,
    moves: Vec<(Color, Move)>,
    players: (String, String),
}

impl ReplayMode {
    /// 履歴を消費して Replay モードを開始する ( デフォルト設定: 手動進行)．
    #[must_use]
    pub fn new(history: GameHistory) -> Self {
        Self::with_options(history, false, DEFAULT_AUTO_DELAY_MS)
    }

    /// 履歴を消費して Replay モードを開始する．`auto_play` の初期値と再生間隔を指定する．
    #[must_use]
    pub fn with_options(history: GameHistory, auto_play: bool, auto_delay_ms: u64) -> Self {
        let moves = build_move_log(&history);
        Self {
            history,
            cursor: 0,
            auto_play,
            auto_delay_ms: clamp_delay(auto_delay_ms),
            moves,
            players: ("Black".into(), "White".into()),
        }
    }

    /// プレイヤー名を設定する ( 表示用)．
    pub fn set_players(&mut self, black: impl Into<String>, white: impl Into<String>) {
        self.players = (black.into(), white.into());
    }

    /// 終局位置 ( 最終手後) に居るか．
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.cursor == self.history.total_moves()
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
                self.auto_play = false; // 終局に到達したので自動再生を停止
            }
            Action::ToggleAutoPlay => {
                if self.is_finished() && !self.auto_play {
                    // 終局後に再度 auto を ON にしたら先頭から再生
                    self.cursor = 0;
                }
                self.auto_play = !self.auto_play;
            }
            Action::IncreaseDelay => {
                self.auto_delay_ms =
                    clamp_delay(self.auto_delay_ms.saturating_add(AUTO_DELAY_STEP_MS));
            }
            Action::DecreaseDelay => {
                self.auto_delay_ms =
                    clamp_delay(self.auto_delay_ms.saturating_sub(AUTO_DELAY_STEP_MS));
            }
            _ => {}
        }
    }

    /// 自動再生の駆動側から呼ぶヘルパ．1 手進めて，終局に達したら auto_play を OFF にする．
    pub fn auto_advance(&mut self) {
        if self.cursor < self.history.total_moves() {
            self.cursor += 1;
        }
        if self.is_finished() {
            self.auto_play = false;
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
        let message = if self.auto_play {
            format!(
                "Move {}/{} [AUTO {}ms]",
                self.cursor, total, self.auto_delay_ms
            )
        } else {
            format!("Move {}/{}", self.cursor, total)
        };
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

    #[test]
    fn with_options_starts_in_auto_play() {
        let r = ReplayMode::with_options(build_history(), true, 1000);
        assert!(r.auto_play);
        assert_eq!(r.auto_delay_ms, 1000);
        let snap = r.snapshot();
        assert!(
            snap.message.contains("[AUTO 1000ms]"),
            "expected delay in message, got: {}",
            snap.message
        );
    }

    #[test]
    fn delay_clamped_to_bounds() {
        let r = ReplayMode::with_options(build_history(), true, 10);
        assert_eq!(r.auto_delay_ms, AUTO_DELAY_MIN_MS);
        let r = ReplayMode::with_options(build_history(), true, 1_000_000);
        assert_eq!(r.auto_delay_ms, AUTO_DELAY_MAX_MS);
    }

    #[test]
    fn increase_and_decrease_delay() {
        let mut r = ReplayMode::with_options(build_history(), false, 500);
        r.handle(Action::IncreaseDelay);
        assert_eq!(r.auto_delay_ms, 500 + AUTO_DELAY_STEP_MS);
        r.handle(Action::DecreaseDelay);
        r.handle(Action::DecreaseDelay);
        assert_eq!(r.auto_delay_ms, 500 - AUTO_DELAY_STEP_MS);
    }

    #[test]
    fn delay_does_not_underflow_below_min() {
        let mut r = ReplayMode::with_options(build_history(), true, AUTO_DELAY_MIN_MS);
        for _ in 0..10 {
            r.handle(Action::DecreaseDelay);
        }
        assert_eq!(r.auto_delay_ms, AUTO_DELAY_MIN_MS);
    }

    #[test]
    fn auto_advance_stops_at_end() {
        let mut r = ReplayMode::with_options(build_history(), true, 100);
        for _ in 0..10 {
            r.auto_advance();
        }
        assert!(r.is_finished());
        assert!(!r.auto_play, "auto_play should turn off at the end");
    }

    #[test]
    fn jump_end_stops_auto_play() {
        let mut r = ReplayMode::with_options(build_history(), true, 100);
        r.handle(Action::JumpEnd);
        assert!(r.is_finished());
        assert!(!r.auto_play);
    }

    #[test]
    fn toggle_auto_at_end_restarts_from_beginning() {
        let mut r = ReplayMode::with_options(build_history(), false, 100);
        r.handle(Action::JumpEnd);
        assert!(r.is_finished());
        // 終局位置で auto を ON にすると先頭に戻ってから再生開始
        r.handle(Action::ToggleAutoPlay);
        assert!(r.auto_play);
        assert_eq!(r.cursor(), 0);
    }
}
