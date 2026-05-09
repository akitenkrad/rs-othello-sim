//! # othello-core
//!
//! Othello (Reversi) のコア型・ルール・盤面表現を提供するクレート．
//!
//! ## 主要型
//!
//! - [`Color`] — 黒・白の二値 enum
//! - [`Coord`] — 行・列を持つ座標
//! - [`Move`] — `Place(Coord)` または `Pass`
//! - [`BoardSize`] — 盤面サイズ
//! - [`Bitboard8`] — 8×8 専用 bitboard 実装
//! - [`GenericBoard`] — 任意サイズ ( 4×4 〜 26×26) の汎用実装
//! - [`Board`] — 上記 2 つを切り替えるハイブリッド enum
//! - [`GameState`] — 盤面 + 手番 + 連続パス回数等
//! - [`GameResult`] — 勝者と石数の終局結果
//! - [`OthelloError`] — エラー型
//!
//! ## Bitboard レイアウト
//!
//! `bit_index = row * 8 + col` ( 行 0 列 0 = bit 0，行 7 列 7 = bit 63)．
//! 8 方向シフトは列マスク ( A 列・H 列) で wrap-around を防ぎつつ実装される．

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

/// よく使う型・トレイトを一括で導入するための prelude．
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
