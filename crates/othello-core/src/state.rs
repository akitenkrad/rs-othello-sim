//! [`GameState`] (game state) and [`GameResult`] (terminal result).

use crate::board::{Board, BoardSize};
use crate::color::Color;
use crate::error::OthelloError;
use crate::mv::Move;

/// State of a single game.
///
/// The game is terminal when `consecutive_passes` reaches 2, or when the
/// board is full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    /// The board.
    pub board: Board,
    /// Side to move next.
    pub side_to_move: Color,
    /// Cumulative move number (0-based). Incremented on every `apply_move`.
    pub move_number: u32,
    /// The most recent move (`None` immediately after game start).
    pub last_move: Option<Move>,
    /// Number of consecutive passes (the game is terminal at 2).
    pub consecutive_passes: u8,
}

impl GameState {
    /// Standard initial state (standard initial position with black to move).
    ///
    /// Always succeeds for `BoardSize::STANDARD` (8x8). For other sizes,
    /// `rows` and `cols` must each be even integers in `4..=26`.
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

    /// Shorthand for the standard 8x8 initial state.
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

    /// Whether the game has ended.
    ///
    /// The game is terminal when there have been two consecutive passes
    /// **or** the board is full (neither side has a legal move).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.consecutive_passes >= 2 || self.board.is_terminal()
    }

    /// Legal moves for the current side to move.
    #[must_use]
    pub fn legal_moves(&self) -> Vec<Move> {
        self.board.legal_moves(self.side_to_move)
    }

    /// Whether the current side must pass (no legal move for `side_to_move`
    /// while the opponent has at least one).
    #[must_use]
    pub fn must_pass(&self) -> bool {
        !self.board.has_any_legal_move(self.side_to_move)
            && self.board.has_any_legal_move(self.side_to_move.opponent())
    }

    /// Plays one move for the current side.
    ///
    /// - On success, `side_to_move` switches to the opponent, `move_number`
    ///   is incremented, and `last_move` is updated.
    /// - For `Pass`, `consecutive_passes` is incremented; otherwise it is
    ///   reset to 0.
    /// - On illegal moves, returns [`OthelloError`] and leaves the state
    ///   unchanged.
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

    /// Computes the terminal result. Returns `None` if not terminal.
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

/// Terminal result of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameResult {
    /// Winner of the game. `None` for a draw (equal stone counts).
    pub winner: Option<Color>,
    /// Black stone count.
    pub black: u32,
    /// White stone count.
    pub white: u32,
    /// Total number of moves (including passes).
    pub total_moves: u32,
}

impl GameResult {
    /// Whether the game is a draw.
    #[inline]
    #[must_use]
    pub const fn is_draw(&self) -> bool {
        self.winner.is_none()
    }

    /// Score margin from the winner's perspective (winner_stones -
    /// loser_stones). Returns 0 for a draw.
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
