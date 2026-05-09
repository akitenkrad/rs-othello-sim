//! 棋譜の Reader / Writer 抽象 trait．

use crate::error::IoError;
use crate::record::GameRecord;
use std::io::{Read, Write};

/// 棋譜 Reader trait．
pub trait GameRecordReader {
    /// 単一の棋譜を読み込む．
    fn read_game<R: Read>(&mut self, source: R) -> Result<GameRecord, IoError>;

    /// 複数の棋譜を読み込む ( デフォルトは `read_game` を 1 回呼ぶ)．
    fn read_all<R: Read>(&mut self, source: R) -> Result<Vec<GameRecord>, IoError> {
        Ok(vec![self.read_game(source)?])
    }
}

/// 棋譜 Writer trait．
pub trait GameRecordWriter {
    /// 単一の棋譜を書き出す．
    fn write_game<W: Write>(&mut self, dest: W, record: &GameRecord) -> Result<(), IoError>;
}
