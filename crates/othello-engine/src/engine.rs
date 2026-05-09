//! [`GameEngine`]: 1 局のゲームループ実行．

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

/// 着手ごとに呼ばれるロギングコールバックの型．Phase 3 の `tracing` 統合と並存する後方互換 API．
pub type LogCallback = Box<dyn FnMut(&str) + Send>;

/// `GameEngine` 設定．
pub struct EngineConfig {
    /// 盤面サイズ．
    pub board_size: BoardSize,
    /// ゲーム ID ( 未指定なら UUID v4 が割り当てられる)．
    pub game_id: Option<String>,
    /// 安全装置: この手数を超えたら強制終了する ( デフォルト `None` で無制限)．
    pub max_moves: Option<u32>,
    /// 着手ごとのコールバック ( ロギング用，後方互換 API)．
    pub log_callback: Option<LogCallback>,
    /// JSONL ロガー ( Phase 3 で追加)．設定すると `game_start` / `move` / `pass` / `game_end` を
    /// 設計書 §4.2 のフォーマットで書き出す．
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
    /// 標準 8×8 設定．
    #[must_use]
    pub fn standard() -> Self {
        Self::default()
    }

    /// 盤面サイズを指定して設定を作る．
    #[must_use]
    pub fn with_size(size: BoardSize) -> Self {
        Self {
            board_size: size,
            ..Self::default()
        }
    }
}

/// `GameEngine` が返すエラー．
#[derive(Debug, Error)]
pub enum EngineError {
    /// プレイヤー側のエラー．
    #[error("player error: {0}")]
    Player(#[from] PlayerError),

    /// ルール違反 ( 不正手など)．
    #[error("rule error: {0}")]
    Rule(#[from] OthelloError),

    /// 設定が不正．
    #[error("config error: {0}")]
    Config(String),

    /// 安全装置の発動 ( `max_moves` 超過)．
    #[error("max moves exceeded: {limit}")]
    MaxMovesExceeded {
        /// 上限値．
        limit: u32,
    },

    /// プレイヤーが合法手を持つのに `Move::Pass` を返した．
    #[error("player attempted to pass while legal moves exist")]
    UnexpectedPass,

    /// JSONL ロガー書き込み失敗．
    #[error("jsonl logger error: {0}")]
    Logger(#[from] othello_io::IoError),
}

/// 1 局を実行するゲームエンジン．
pub struct GameEngine {
    state: GameState,
    history: GameHistory,
    config: EngineConfig,
    started_at: DateTime<FixedOffset>,
    ended_at: Option<DateTime<FixedOffset>>,
}

impl GameEngine {
    /// 設定からエンジンを生成する．
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

    /// 1 局を実行する ( 既存 API)．
    ///
    /// アルゴリズムは設計書 §3.3.3 の pseudocode に従う．
    /// プレイヤーが合法手を持つのに Pass を返した場合は [`EngineError::UnexpectedPass`]．
    /// 合法手が無い ( pass しか取れない) 場合は engine 側で `Move::Pass` を強制する．
    ///
    /// JSONL ロガー / tracing スパンは内部の `players` 情報を `<unknown>` として記録する．
    /// プレイヤー名を含めて出力したい場合は [`Self::run_with_meta`] を使う．
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

    /// 1 局を実行し，JSONL/tracing にプレイヤー名を渡せる版．
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

    /// 履歴の参照を返す．
    #[inline]
    #[must_use]
    pub fn history(&self) -> &GameHistory {
        &self.history
    }

    /// 現在の `GameState`．
    #[inline]
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// 設定の参照．
    #[inline]
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// 履歴を消費して `GameRecord` に変換する．
    ///
    /// `players` には実際に対戦した黒白プレイヤーの情報を渡す ( name + params)．
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
