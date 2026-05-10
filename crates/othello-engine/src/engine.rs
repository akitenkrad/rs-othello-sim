//! [`GameEngine`]: drives the game loop for a single game.

use crate::history::GameHistory;
use chrono::{DateTime, FixedOffset, Local, Offset, Utc};
use othello_core::{BoardSize, Color, GameResult, GameState, Move, OthelloError};
use othello_io::{
    GameEndEvent, GameMetadata, GameRecord, GameResultRecord, GameStartEvent, JsonlLogger,
    MoveEntry, MoveEvent, PassEvent, PlayerInfo, PlayerNames, PlayerPair, SCHEMA_VERSION, Score,
    Stones,
};
use othello_player::{Player, PlayerError};
use thiserror::Error;
use tracing::{debug, info, info_span};

/// Type of the per-move logging callback. A backwards-compatible API kept
/// alongside the `tracing` integration introduced in Phase 3.
pub type LogCallback = Box<dyn FnMut(&str) + Send>;

/// Configuration for [`GameEngine`].
pub struct EngineConfig {
    /// Board size.
    pub board_size: BoardSize,
    /// Game ID. A UUID v4 is generated automatically if `None`.
    pub game_id: Option<String>,
    /// Safety cap: abort once this many moves have been played. `None`
    /// disables the cap.
    pub max_moves: Option<u32>,
    /// Per-move callback for logging (backwards-compatible API).
    pub log_callback: Option<LogCallback>,
    /// JSONL logger (added in Phase 3). When set, emits `game_start` /
    /// `move` / `pass` / `game_end` records in the format specified by the
    /// design document §4.2.
    pub jsonl_logger: Option<JsonlLogger>,
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("board_size", &self.board_size)
            .field("game_id", &self.game_id)
            .field("max_moves", &self.max_moves)
            .field("log_callback", &self.log_callback.as_ref().map(|_| "<fn>"))
            .field("jsonl_logger", &self.jsonl_logger)
            .finish()
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            board_size: BoardSize::STANDARD,
            game_id: None,
            max_moves: None,
            log_callback: None,
            jsonl_logger: None,
        }
    }
}

impl EngineConfig {
    /// Default 8x8 configuration.
    #[must_use]
    pub fn standard() -> Self {
        Self::default()
    }

    /// Builds a config with an explicit board size.
    #[must_use]
    pub fn with_size(size: BoardSize) -> Self {
        Self {
            board_size: size,
            ..Self::default()
        }
    }
}

/// Errors returned by [`GameEngine`].
#[derive(Debug, Error)]
pub enum EngineError {
    /// Error from a player.
    #[error("player error: {0}")]
    Player(#[from] PlayerError),

    /// Rule violation (e.g. illegal move).
    #[error("rule error: {0}")]
    Rule(#[from] OthelloError),

    /// Invalid configuration.
    #[error("config error: {0}")]
    Config(String),

    /// Safety cap tripped (`max_moves` exceeded).
    #[error("max moves exceeded: {limit}")]
    MaxMovesExceeded {
        /// The cap value.
        limit: u32,
    },

    /// A player returned `Move::Pass` while legal moves existed.
    #[error("player attempted to pass while legal moves exist")]
    UnexpectedPass,

    /// JSONL logger write failed.
    #[error("jsonl logger error: {0}")]
    Logger(#[from] othello_io::IoError),
}

/// Game engine that executes a single game.
pub struct GameEngine {
    state: GameState,
    history: GameHistory,
    config: EngineConfig,
    started_at: DateTime<FixedOffset>,
    ended_at: Option<DateTime<FixedOffset>>,
}

impl GameEngine {
    /// Builds an engine from the given configuration.
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let initial = GameState::standard(config.board_size).map_err(EngineError::from)?;
        let history = GameHistory::new(initial.clone());
        Ok(Self {
            state: initial,
            history,
            config,
            started_at: now(),
            ended_at: None,
        })
    }

    /// Runs a single game (legacy API).
    ///
    /// The algorithm follows the pseudocode in §3.3.3 of the design
    /// document. Returns [`EngineError::UnexpectedPass`] if a player
    /// returns `Pass` while legal moves exist. When no legal move is
    /// available (the side can only pass), the engine substitutes
    /// `Move::Pass` automatically.
    ///
    /// The JSONL logger / tracing span record the players as
    /// `<unknown>`. Use [`Self::run_with_meta`] to include the actual
    /// player names in the output.
    pub fn run<B: Player, W: Player>(
        &mut self,
        black: &mut B,
        white: &mut W,
    ) -> Result<GameResult, EngineError> {
        self.run_with_meta(
            black,
            white,
            PlayerPair {
                black: PlayerInfo::just_name("<unknown>"),
                white: PlayerInfo::just_name("<unknown>"),
            },
        )
    }

    /// Runs a single game and forwards player names to the JSONL logger
    /// and tracing spans.
    pub fn run_with_meta<B: Player, W: Player>(
        &mut self,
        black: &mut B,
        white: &mut W,
        players: PlayerPair,
    ) -> Result<GameResult, EngineError> {
        black.reset();
        white.reset();

        self.started_at = now();

        let game_id = self.game_id_or_uuid();
        let span = info_span!(
            "game",
            id = %game_id,
            rows = self.config.board_size.rows,
            cols = self.config.board_size.cols,
        );
        let _enter = span.enter();
        info!(
            event = "game_start",
            black = %players.black.name,
            white = %players.white.name,
            "game start"
        );

        // JSONL: game_start
        if let Some(logger) = self.config.jsonl_logger.as_mut() {
            logger.log_game_start(&GameStartEvent {
                ts: self.started_at,
                game_id: game_id.clone(),
                board_size: [self.config.board_size.rows, self.config.board_size.cols],
                players: PlayerNames {
                    black: players.black.name.clone(),
                    white: players.white.name.clone(),
                },
            })?;
        }

        self.log_text(&format!(
            "game_start id={game_id} board={:?}",
            self.config.board_size
        ));

        while !self.state.is_terminal() {
            // 安全装置
            if let Some(limit) = self.config.max_moves
                && self.state.move_number >= limit
            {
                return Err(EngineError::MaxMovesExceeded { limit });
            }

            // 合法手なし → Engine が Pass を強制
            let must_pass = self.state.legal_moves().is_empty();
            let side_pre = self.state.side_to_move;
            let legal_count = self.state.legal_moves().len() as u32;
            let mv: Move = if must_pass {
                Move::Pass
            } else {
                let player: &mut dyn Player = if self.state.side_to_move == black.color() {
                    black
                } else if self.state.side_to_move == white.color() {
                    white
                } else {
                    return Err(EngineError::Config(format!(
                        "no player matches side_to_move={:?} (black={:?}, white={:?})",
                        self.state.side_to_move,
                        black.color(),
                        white.color(),
                    )));
                };
                let chosen = player.select_move(&self.state)?;
                if matches!(chosen, Move::Pass) {
                    return Err(EngineError::UnexpectedPass);
                }
                chosen
            };

            // 適用
            self.state.apply_move(mv)?;
            self.history.push(mv, self.state.clone());

            let stones = Stones {
                black: self.state.board.count(Color::Black),
                white: self.state.board.count(Color::White),
            };

            // tracing + JSONL
            let n = self.state.move_number;
            let ts = now();
            match mv {
                Move::Place(c) => {
                    debug!(
                        event = "move",
                        n,
                        side = ?side_pre,
                        move_row = c.row,
                        move_col = c.col,
                        legal_count,
                        "place"
                    );
                    if let Some(logger) = self.config.jsonl_logger.as_mut() {
                        logger.log_move(&MoveEvent {
                            ts,
                            game_id: game_id.clone(),
                            n,
                            side: side_pre,
                            r#move: mv.into(),
                            stones,
                            legal_count,
                        })?;
                    }
                    self.log_text(&format!(
                        "move n={n} side={side_pre:?} mv={mv:?} stones=({},{})",
                        stones.black, stones.white
                    ));
                }
                Move::Pass => {
                    debug!(event = "pass", n, side = ?side_pre, "pass");
                    if let Some(logger) = self.config.jsonl_logger.as_mut() {
                        logger.log_pass(&PassEvent {
                            ts,
                            game_id: game_id.clone(),
                            n,
                            side: side_pre,
                        })?;
                    }
                    self.log_text(&format!("pass n={n} side={side_pre:?}"));
                }
            }
        }

        let result = self
            .state
            .result()
            .expect("terminal state must have result");
        self.ended_at = Some(now());

        info!(
            event = "game_end",
            winner = ?result.winner,
            black = result.black,
            white = result.white,
            moves_total = result.total_moves,
            "game end"
        );

        if let Some(logger) = self.config.jsonl_logger.as_mut() {
            logger.log_game_end(&GameEndEvent {
                ts: self.ended_at.unwrap(),
                game_id: game_id.clone(),
                winner: result.winner,
                stones: Stones {
                    black: result.black,
                    white: result.white,
                },
                moves_total: result.total_moves,
            })?;
            logger.flush()?;
        }

        self.log_text(&format!(
            "game_end id={game_id} winner={:?} score=({},{})",
            result.winner, result.black, result.white
        ));

        black.on_game_end(&self.state, result);
        white.on_game_end(&self.state, result);

        Ok(result)
    }

    /// Returns a reference to the history.
    #[inline]
    #[must_use]
    pub fn history(&self) -> &GameHistory {
        &self.history
    }

    /// Returns the current `GameState`.
    #[inline]
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Returns a reference to the configuration.
    #[inline]
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Consumes the history and converts it into a `GameRecord`.
    ///
    /// Pass the actual black/white player information (name + params) in
    /// `players`.
    pub fn into_record(self, players: PlayerPair) -> GameRecord {
        let metadata = GameMetadata {
            id: self
                .config
                .game_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            started_at: self.started_at,
            ended_at: self.ended_at,
            board_size: self.config.board_size,
            players,
            result: self.state.result().map(|r| GameResultRecord {
                winner: r.winner,
                score: Score {
                    black: r.black,
                    white: r.white,
                },
            }),
            engine_version: format!("rs-othello-sim {}", env!("CARGO_PKG_VERSION")),
        };

        let mut moves: Vec<MoveEntry> = Vec::with_capacity(self.history.total_moves());
        for (i, (mv, post)) in self
            .history
            .moves()
            .iter()
            .zip(self.history.snapshots().iter().skip(1))
            .enumerate()
        {
            moves.push(MoveEntry {
                n: (i + 1) as u32,
                side: post.side_to_move.opponent(),
                r#move: *mv,
                ts: self.started_at,
            });
        }

        GameRecord {
            schema_version: SCHEMA_VERSION.to_string(),
            metadata,
            moves,
        }
    }

    fn log_text(&mut self, msg: &str) {
        if let Some(cb) = self.config.log_callback.as_mut() {
            cb(msg);
        }
    }

    fn game_id_or_uuid(&self) -> String {
        self.config
            .game_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }
}

fn now() -> DateTime<FixedOffset> {
    let local = Local::now();
    local.with_timezone(&local.offset().fix())
}

// 上記 now() は `Local` のオフセットを `FixedOffset` に固定する．Local が利用できない環境では UTC ( +00:00) になる．
#[allow(dead_code)]
fn _now_utc() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::Color;
    use othello_io::PlayerInfo;
    use othello_player::{GreedyPlayer, RandomPlayer};

    #[test]
    fn random_vs_random_terminates() {
        let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
        let mut black = RandomPlayer::with_seed(Color::Black, 1);
        let mut white = RandomPlayer::with_seed(Color::White, 2);
        let result = engine.run(&mut black, &mut white).unwrap();
        assert!(result.black + result.white <= 64);
        assert!(engine.state().is_terminal());
    }

    #[test]
    fn history_length_consistent() {
        let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
        let mut black = RandomPlayer::with_seed(Color::Black, 1);
        let mut white = GreedyPlayer::with_color(Color::White);
        engine.run(&mut black, &mut white).unwrap();
        assert_eq!(
            engine.history().snapshots().len(),
            engine.history().total_moves() + 1
        );
    }

    #[test]
    fn into_record_produces_valid_record() {
        let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
        let mut black = RandomPlayer::with_seed(Color::Black, 7);
        let mut white = RandomPlayer::with_seed(Color::White, 8);
        engine.run(&mut black, &mut white).unwrap();
        let record = engine.into_record(PlayerPair {
            black: PlayerInfo::just_name("RandomPlayer"),
            white: PlayerInfo::just_name("RandomPlayer"),
        });
        assert!(record.moves.len() > 10);
        assert!(record.metadata.result.is_some());
    }

    #[test]
    fn config_with_swapped_player_colors_errors() {
        let mut engine = GameEngine::new(EngineConfig::standard()).unwrap();
        let mut black = RandomPlayer::with_seed(Color::Black, 1);
        let mut wrong_white = RandomPlayer::with_seed(Color::Black, 2);
        let r = engine.run(&mut black, &mut wrong_white);
        assert!(matches!(r, Err(EngineError::Config(_))));
    }

    #[test]
    fn jsonl_logger_emits_start_move_end() {
        // Cursor<Vec<u8>> 経由ではなく，一時ファイルに書いて行数確認．
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "rs-othello-sim-engine-jsonl-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let logger = JsonlLogger::to_path(&path).unwrap();
            let mut config = EngineConfig::standard();
            config.jsonl_logger = Some(logger);
            let mut engine = GameEngine::new(config).unwrap();
            let mut black = RandomPlayer::with_seed(Color::Black, 1);
            let mut white = RandomPlayer::with_seed(Color::White, 2);
            engine
                .run_with_meta(
                    &mut black,
                    &mut white,
                    PlayerPair {
                        black: PlayerInfo::just_name("RandomPlayer"),
                        white: PlayerInfo::just_name("RandomPlayer"),
                    },
                )
                .unwrap();
        }
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert!(lines.len() >= 3, "want at least start/move/end events");
        assert!(lines[0].starts_with("{\"event\":\"game_start\""));
        assert!(lines.last().unwrap().starts_with("{\"event\":\"game_end\""));
        // 中央のいずれかは move か pass
        let inner = &lines[1..lines.len() - 1];
        assert!(
            inner
                .iter()
                .all(|l| l.contains("\"event\":\"move\"") || l.contains("\"event\":\"pass\""))
        );
        // PlayerNames が反映されている
        assert!(lines[0].contains("\"black\":\"RandomPlayer\""));
        let _ = std::fs::remove_file(&path);
    }
}
