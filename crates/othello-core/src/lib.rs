//! # othello-core
//!
//! Core types, rules, and board representations for Othello (Reversi).
//!
//! ## Key types
//!
//! - [`Color`] — Black/White enum.
//! - [`Coord`] — Row/column coordinate.
//! - [`Move`] — `Place(Coord)` or `Pass`.
//! - [`BoardSize`] — Board dimensions.
//! - [`Bitboard8`] — Bitboard implementation specialized to 8x8.
//! - [`GenericBoard`] — Generic implementation for arbitrary sizes (4x4 to 26x26).
//! - [`Board`] — Hybrid enum that switches between the two implementations above.
//! - [`GameState`] — Board, side-to-move, consecutive pass count, etc.
//! - [`GameResult`] — Winner and stone counts at the end of the game.
//! - [`OthelloError`] — Error type.
//!
//! ## Bitboard layout
//!
//! `bit_index = row * 8 + col` (row 0 col 0 = bit 0, row 7 col 7 = bit 63).
//! Eight-direction shifts use column masks (column A, column H) to prevent
//! wrap-around.

pub mod bitboard;
pub mod board;
pub mod color;
pub mod coord;
pub mod error;
pub mod generic_board;
pub mod mv;
pub mod rules;
pub mod state;

pub use bitboard::Bitboard8;
pub use board::{Board, BoardSize};
pub use color::Color;
pub use coord::Coord;
pub use error::OthelloError;
pub use generic_board::GenericBoard;
pub use mv::Move;
pub use state::{GameResult, GameState};

/// Prelude that imports the commonly used types and traits in one go.
///
/// ```
/// use othello_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        Bitboard8, Board, BoardSize, Color, Coord, GameResult, GameState, GenericBoard, Move,
        OthelloError,
    };
}
