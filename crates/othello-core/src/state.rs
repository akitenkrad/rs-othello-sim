//! ゲーム状態 [`GameState`] と終局結果 [`GameResult`]．

use crate::board::{Board, BoardSize};
use crate::color::Color;
use crate::error::OthelloError;
use crate::mv::Move;

/// 1 局のゲーム状態．
///
/// `consecutive_passes` が 2 になると終局．盤面満杯でも終局．
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    /// 盤面．
    pub board: Board,
    /// 次に着手するプレイヤー色．
    pub side_to_move: Color,
    /// 累計手数 ( 0 起点)．`apply_move` のたびに 1 増える．
    pub move_number: u32,
    /// 直前の着手 ( ゲーム開始直後は `None`)．
    pub last_move: Option<Move>,
    /// 連続したパスの回数 ( 2 で終局判定)．
    pub consecutive_passes: u8,
}

impl GameState {
    /// 標準的な初期状態 ( 標準初期配置 + 黒の手番)．
    ///
    /// `BoardSize::STANDARD` ( 8×8) なら必ず成功．他のサイズは
    /// rows / cols が 4..=26 の偶数である必要がある．
    pub fn standard(size: BoardSize) -> Result<Self, OthelloError> {
        let board = Board::standard(size)?;
        Ok(Self {
            board,
            side_to_move: Color::Black,
            move_number: 0,
            last_move: None,
            consecutive_passes: 0,
        })
    }

    /// 標準 8×8 初期状態の shorthand．
    #[must_use]
    pub fn standard_8x8() -> Self {
        Self {
            board: Board::standard_8x8(),
            side_to_move: Color::Black,
            move_number: 0,
            last_move: None,
            consecutive_passes: 0,
        }
    }

    /// 終局かどうか．
    ///
    /// 連続パス 2 回 **または** 盤面満杯 ( 両者合法手なし) で終局．
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.consecutive_passes >= 2 || self.board.is_terminal()
    }

    /// 現手番の合法手リスト．
    #[must_use]
    pub fn legal_moves(&self) -> Vec<Move> {
        self.board.legal_moves(self.side_to_move)
    }

    /// 現手番が Pass しか取れない局面かどうか ( 合法手 0 件かつ相手は合法手あり)．
    #[must_use]
    pub fn must_pass(&self) -> bool {
        !self.board.has_any_legal_move(self.side_to_move)
            && self.board.has_any_legal_move(self.side_to_move.opponent())
    }

    /// 現手番として 1 手指す．
    ///
    /// - 着手成功時，`side_to_move` は相手に移り，`move_number` は +1，`last_move` 更新
    /// - Pass 時は `consecutive_passes` を +1，それ以外は 0 にリセット
    /// - 不正手は [`OthelloError`] を返し，state は変更しない
    pub fn apply_move(&mut self, mv: Move) -> Result<Vec<crate::Coord>, OthelloError> {
        let flips = self.board.apply(self.side_to_move, mv)?;
        match mv {
            Move::Pass => {
                self.consecutive_passes = self.consecutive_passes.saturating_add(1);
            }
            Move::Place(_) => {
                self.consecutive_passes = 0;
            }
        }
        self.last_move = Some(mv);
        self.move_number = self.move_number.saturating_add(1);
        self.side_to_move = self.side_to_move.opponent();
        Ok(flips)
    }

    /// 終局結果を計算する．終局でない場合は `None`．
    #[must_use]
    pub fn result(&self) -> Option<GameResult> {
        if !self.is_terminal() {
            return None;
        }
        let black = self.board.count(Color::Black);
        let white = self.board.count(Color::White);
        let winner = match black.cmp(&white) {
            std::cmp::Ordering::Greater => Some(Color::Black),
            std::cmp::Ordering::Less => Some(Color::White),
            std::cmp::Ordering::Equal => None,
        };
        Some(GameResult {
            winner,
            black,
            white,
            total_moves: self.move_number,
        })
    }
}

/// 終局結果．
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameResult {
    /// 勝者．同数なら `None` ( 引き分け)．
    pub winner: Option<Color>,
    /// 黒石数．
    pub black: u32,
    /// 白石数．
    pub white: u32,
    /// 総手数 ( パス含む)．
    pub total_moves: u32,
}

impl GameResult {
    /// 引き分けかどうか．
    #[inline]
    #[must_use]
    pub const fn is_draw(&self) -> bool {
        self.winner.is_none()
    }

    /// 勝者から見た得点差 ( 勝者石数 - 敗者石数)．引き分けなら 0．
    #[inline]
    #[must_use]
    pub const fn margin(&self) -> i32 {
        let b = self.black as i32;
        let w = self.white as i32;
        match self.winner {
            Some(Color::Black) => b - w,
            Some(Color::White) => w - b,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Coord;

    #[test]
    fn standard_state_initial_conditions() {
        let s = GameState::standard_8x8();
        assert_eq!(s.side_to_move, Color::Black);
        assert_eq!(s.move_number, 0);
        assert_eq!(s.last_move, None);
        assert_eq!(s.consecutive_passes, 0);
        assert!(!s.is_terminal());
        assert_eq!(s.legal_moves().len(), 4);
    }

    #[test]
    fn apply_move_advances_state() {
        let mut s = GameState::standard_8x8();
        s.apply_move(Move::Place(Coord::new(2, 3))).unwrap();
        assert_eq!(s.side_to_move, Color::White);
        assert_eq!(s.move_number, 1);
        assert_eq!(s.last_move, Some(Move::Place(Coord::new(2, 3))));
        assert_eq!(s.consecutive_passes, 0);
    }

    #[test]
    fn pass_increments_consecutive_passes() {
        // パス可能局面を作る: 全マス黒
        let mut s = GameState::standard_8x8();
        // 強制的に盤面を全黒にする
        for r in 0..8u8 {
            for c in 0..8u8 {
                s.board.set(Coord::new(r, c), Some(Color::Black));
            }
        }
        // 白の手番に変える
        s.side_to_move = Color::White;
        s.apply_move(Move::Pass).unwrap();
        assert_eq!(s.consecutive_passes, 1);
    }

    #[test]
    fn result_returns_none_when_not_terminal() {
        let s = GameState::standard_8x8();
        assert_eq!(s.result(), None);
    }

    #[test]
    fn result_when_terminal() {
        let mut s = GameState::standard_8x8();
        // 全マス黒にしてから両側 Pass を発生させる
        for r in 0..8u8 {
            for c in 0..8u8 {
                s.board.set(Coord::new(r, c), Some(Color::Black));
            }
        }
        s.consecutive_passes = 2;
        let result = s.result().unwrap();
        assert_eq!(result.winner, Some(Color::Black));
        assert_eq!(result.black, 64);
        assert_eq!(result.white, 0);
        assert_eq!(result.margin(), 64);
    }

    #[test]
    fn draw_result() {
        let result = GameResult {
            winner: None,
            black: 32,
            white: 32,
            total_moves: 60,
        };
        assert!(result.is_draw());
        assert_eq!(result.margin(), 0);
    }
}
