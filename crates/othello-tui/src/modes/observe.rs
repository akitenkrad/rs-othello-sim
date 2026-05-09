//! Observe モード ( AI 対戦観戦) のロジック．

use crate::app::{AppMode, AppState, Cursor, format_move_history};
use crate::input::Action;
use othello_core::{BoardSize, Color, GameState, Move, OthelloError};
use othello_player::Player;

/// Observe モードのバックエンド設定．
pub struct ObserveBackend {
    /// 黒プレイヤー ( 内部で `select_move` が呼ばれる)．
    pub black: Box<dyn Player>,
    /// 白プレイヤー．
    pub white: Box<dyn Player>,
    /// 表示用プレイヤー名 ( black, white)．
    pub names: (String, String),
}

/// Observe モード状態．
pub struct ObserveMode {
    state: GameState,
    moves: Vec<(Color, Move)>,
    backend: ObserveBackend,
    /// 自動再生間隔 ( ms)．`0` は手動進行のみ．
    pub auto_delay_ms: u64,
    /// 自動再生中フラグ．
    pub auto_play: bool,
    finished: bool,
    message: String,
}

impl ObserveMode {
    /// 標準初期局面で Observe モードを開始する．
    pub fn new(
        board_size: BoardSize,
        backend: ObserveBackend,
        auto_delay_ms: u64,
    ) -> Result<Self, OthelloError> {
        let state = GameState::standard(board_size)?;
        let auto_play = auto_delay_ms > 0;
        let message = if auto_play {
            format!("Auto-play [{}ms]", auto_delay_ms)
        } else {
            format!("{:?} to move ( Space=step)", state.side_to_move)
        };
        Ok(Self {
            state,
            moves: Vec::new(),
            backend,
            auto_delay_ms,
            auto_play,
            finished: false,
            message,
        })
    }

    /// 1 アクションを処理する．
    pub fn handle(&mut self, action: Action) {
        match action {
            Action::StepForward => {
                let _ = self.advance_one();
            }
            Action::ToggleAutoPlay => {
                self.auto_play = !self.auto_play;
                self.message = if self.auto_play {
                    format!("Auto-play [{}ms]", self.auto_delay_ms.max(1))
                } else {
                    format!("{:?} to move ( Space=step)", self.state.side_to_move)
                };
            }
            Action::IncreaseDelay => {
                self.auto_delay_ms = self.auto_delay_ms.saturating_add(100);
                self.message = format!("Delay {} ms", self.auto_delay_ms);
            }
            Action::DecreaseDelay => {
                self.auto_delay_ms = self.auto_delay_ms.saturating_sub(100);
                self.message = format!("Delay {} ms", self.auto_delay_ms);
            }
            _ => {}
        }
    }

    /// 1 手だけ進める ( 内部状態を更新)．戻り値: 進めたか．
    pub fn advance_one(&mut self) -> bool {
        if self.finished {
            return false;
        }
        if self.state.is_terminal() {
            self.finished = true;
            return false;
        }
        let side = self.state.side_to_move;
        let mv = if self.state.legal_moves().is_empty() {
            Move::Pass
        } else {
            let result = match side {
                Color::Black => self.backend.black.select_move(&self.state),
                Color::White => self.backend.white.select_move(&self.state),
            };
            match result {
                Ok(m) => m,
                Err(e) => {
                    self.message = format!("Player error: {e}");
                    self.finished = true;
                    return false;
                }
            }
        };
        match self.state.apply_move(mv) {
            Ok(_) => {
                self.moves.push((side, mv));
                if self.state.is_terminal() {
                    self.finished = true;
                    if let Some(r) = self.state.result() {
                        self.message = format!(
                            "Game over: black={} white={} winner={:?}",
                            r.black, r.white, r.winner
                        );
                    }
                } else {
                    self.message = format!(
                        "Move {}: {:?} played ( {:?} to move)",
                        self.state.move_number, side, self.state.side_to_move
                    );
                }
                true
            }
            Err(e) => {
                self.message = format!("Illegal move from player: {e}");
                self.finished = true;
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
        let cursor = Cursor::center(&self.state);
        AppState {
            mode: AppMode::Observe,
            state: self.state.clone(),
            cursor,
            message: self.message.clone(),
            players: self.backend.names.clone(),
            move_history: format_move_history(&self.moves, self.moves.len()),
            move_cursor: self.moves.len(),
            total_moves: self.moves.len(),
            auto_play: self.auto_play,
            finished: self.finished,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::Color;
    use othello_player::RandomPlayer;

    fn make_backend() -> ObserveBackend {
        ObserveBackend {
            black: Box::new(RandomPlayer::with_seed(Color::Black, 1)),
            white: Box::new(RandomPlayer::with_seed(Color::White, 2)),
            names: ("Random[1]".into(), "Random[2]".into()),
        }
    }

    #[test]
    fn advance_progresses_state() {
        let mut o = ObserveMode::new(BoardSize::STANDARD, make_backend(), 0).unwrap();
        assert!(o.advance_one());
        assert_eq!(o.state.move_number, 1);
    }

    #[test]
    fn run_to_terminal() {
        let mut o = ObserveMode::new(BoardSize::STANDARD, make_backend(), 0).unwrap();
        let mut safety = 200;
        while !o.is_finished() && safety > 0 {
            o.advance_one();
            safety -= 1;
        }
        assert!(o.state.is_terminal());
    }

    #[test]
    fn toggle_autoplay() {
        let mut o = ObserveMode::new(BoardSize::STANDARD, make_backend(), 0).unwrap();
        assert!(!o.auto_play);
        o.handle(Action::ToggleAutoPlay);
        assert!(o.auto_play);
    }

    #[test]
    fn snapshot_observable_fields() {
        let o = ObserveMode::new(BoardSize::STANDARD, make_backend(), 250).unwrap();
        let snap = o.snapshot();
        assert_eq!(snap.mode, AppMode::Observe);
        assert!(snap.auto_play);
        assert_eq!(snap.players.0, "Random[1]");
    }
}
