//! Reader / Writer abstraction traits for game records.

use crate::error::IoError;
use crate::record::GameRecord;
use std::io::{Read, Write};

/// Game-record reader trait.
pub trait GameRecordReader {
    /// Reads a single game record.
    fn read_game<R: Read>(&mut self, source: R) -> Result<GameRecord, IoError>;

    /// Reads multiple game records. The default implementation calls
    /// `read_game` once.
    fn read_all<R: Read>(&mut self, source: R) -> Result<Vec<GameRecord>, IoError> {
        Ok(vec![self.read_game(source)?])
    }
}

/// Game-record writer trait.
pub trait GameRecordWriter {
    /// Writes a single game record.
    fn write_game<W: Write>(&mut self, dest: W, record: &GameRecord) -> Result<(), IoError>;
}
