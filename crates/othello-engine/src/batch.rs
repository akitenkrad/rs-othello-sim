//! [`BatchRunner`]: 複数局のゲームを `rayon` で並列実行する．
//!
//! 設計書 §3.3.4 準拠．
//!
//! ## 主な機能
//!
//! - スレッド数指定 ( 0 = 論理コア数)
//! - 各ゲームへの seed 派生 ( ベース seed + ゲーム index)
//! - `swap_colors` で偶数/奇数局の黒白入れ替え
//! - 各ゲームの JSON 棋譜出力 (`log_dir`)
//! - 全局集約 JSONL ログ (`jsonl_log_path`)

use crate::engine::{EngineConfig, EngineError, GameEngine};
use crate::history::GameHistory;
use chrono::{DateTime, FixedOffset, Local, Offset};
use othello_core::{BoardSize, Color, GameResult};
use othello_io::{
    GameEndEvent, GameMetadata, GameRecord, GameRecordWriter, GameResultRecord, GameStartEvent,
    JsonWriter, MoveEntry, MoveEvent, PassEvent, PlayerInfo, PlayerNames, PlayerPair,
    SCHEMA_VERSION, Score, Stones,
};
use othello_player::{Player, PlayerError};
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

/// バッチ実行の進捗コールバック．各ゲーム完了時に 1 回呼ばれる．
///
/// `BatchRunner.run` の中で `rayon` の並列スレッドから呼ばれるため，実装は
/// 内部で同期化されている必要がある ( `Mutex` 等で囲む)．
pub trait ProgressCallback: Send + Sync {
    /// `index` 番目 ( 0 起点) のゲームが完了したときに呼ばれる．
    fn on_game_complete(&self, game_index: usize, summary: &GameSummary);
}

/// バッチ実行設定．
pub struct BatchConfig {
    /// 試合数．
    pub num_games: usize,
    /// 並列スレッド数 ( 0 = 論理コア数を rayon に委ねる)．
    pub num_threads: usize,
    /// ベース seed ( 各ゲームには `seed + game_index` が渡る)．
    pub seed: Option<u64>,
    /// 各ゲームの JSON 棋譜を `<dir>/game_{index:08}.json` で保存する．
    pub log_dir: Option<PathBuf>,
    /// 偶奇でプレイヤー色を入れ替える．
    pub swap_colors: bool,
    /// 盤面サイズ．
    pub board_size: BoardSize,
    /// 安全装置．
    pub max_moves: Option<u32>,
    /// 全局イベントを 1 ファイルに集約する JSONL ログ．
    pub jsonl_log_path: Option<PathBuf>,
    /// 各ゲーム完了時に呼ばれるコールバック ( 進捗バー等)．
    pub progress: Option<Arc<dyn ProgressCallback>>,
}

impl std::fmt::Debug for BatchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchConfig")
            .field("num_games", &self.num_games)
            .field("num_threads", &self.num_threads)
            .field("seed", &self.seed)
            .field("log_dir", &self.log_dir)
            .field("swap_colors", &self.swap_colors)
            .field("board_size", &self.board_size)
            .field("max_moves", &self.max_moves)
            .field("jsonl_log_path", &self.jsonl_log_path)
            .field("progress", &self.progress.as_ref().map(|_| "<callback>"))
            .finish()
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            num_games: 1,
            num_threads: 0,
            seed: None,
            log_dir: None,
            swap_colors: false,
            board_size: BoardSize::STANDARD,
            max_moves: None,
            jsonl_log_path: None,
            progress: None,
        }
    }
}

/// 1 局のサマリ．
#[derive(Debug, Clone)]
pub struct GameSummary {
    /// ゲーム ID ( UUID v4 文字列または `factory` index)．
    pub game_id: String,
    /// 黒石数．
    pub black_score: u32,
    /// 白石数．
    pub white_score: u32,
    /// 勝者 ( 引き分けは `None`)．
    pub winner: Option<Color>,
    /// 総手数．
    pub total_moves: u32,
    /// `swap_colors=true` で実際に入れ替えたか ( この局のみ)．
    pub colors_swapped: bool,
}

/// バッチ実行結果．
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// 完了局数．
    pub num_games: usize,
    /// 黒勝利数．
    pub black_wins: usize,
    /// 白勝利数．
    pub white_wins: usize,
    /// 引き分け数．
    pub draws: usize,
    /// 総手数 ( 全局合計)．
    pub total_moves: u64,
    /// 経過時間．
    pub elapsed: Duration,
    /// 各局のサマリ．
    pub per_game: Vec<GameSummary>,
}

/// バッチ実行エラー．
#[derive(Debug, Error)]
pub enum BatchError {
    /// エンジンエラー．
    #[error("engine error in game #{index}: {source}")]
    Engine {
        /// 対象ゲーム index．
        index: usize,
        /// 原因．
        source: EngineError,
    },
    /// プレイヤーエラー．
    #[error("player error: {0}")]
    Player(#[from] PlayerError),
    /// IO エラー．
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// rayon スレッドプール構築失敗．
    #[error("thread pool error: {0}")]
    ThreadPool(String),
}

/// バッチ実行器．
pub struct BatchRunner {
    config: BatchConfig,
}

impl BatchRunner {
    /// 設定からバッチ実行器を生成する．
    #[must_use]
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// 設定への参照．
    #[inline]
    #[must_use]
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// バッチを実行する．
    ///
    /// `factory` は各ゲームの seed を受け取り `(black_player, white_player)` を返す．
    /// `swap_colors == true` のとき，奇数 index のゲームでは戻り値の tuple を逆にして使う．
    pub fn run<F>(&self, factory: F) -> Result<BatchResult, BatchError>
    where
        F: Fn(u64) -> (Box<dyn Player>, Box<dyn Player>) + Sync + Send,
    {
        // log_dir 準備
        if let Some(dir) = &self.config.log_dir {
            create_dir_all(dir)?;
        }
        // jsonl 集約ログ準備
        let jsonl_writer: Option<Mutex<BufWriter<File>>> =
            if let Some(path) = &self.config.jsonl_log_path {
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                {
                    create_dir_all(parent)?;
                }
                let f = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?;
                Some(Mutex::new(BufWriter::new(f)))
            } else {
                None
            };

        let pool = if self.config.num_threads == 0 {
            ThreadPoolBuilder::new().build()
        } else {
            ThreadPoolBuilder::new()
                .num_threads(self.config.num_threads)
                .build()
        }
        .map_err(|e| BatchError::ThreadPool(e.to_string()))?;

        let base_seed = self.config.seed.unwrap_or(0);
        let started = Instant::now();

        let summaries: Vec<Result<GameSummary, BatchError>> = pool.install(|| {
            (0..self.config.num_games)
                .into_par_iter()
                .map(|i| self.run_one(i, base_seed.wrapping_add(i as u64), &factory, &jsonl_writer))
                .collect()
        });

        // jsonl flush
        if let Some(mtx) = jsonl_writer {
            let mut guard = mtx.into_inner().expect("mutex poisoned");
            guard.flush()?;
        }

        let mut result = BatchResult {
            num_games: 0,
            black_wins: 0,
            white_wins: 0,
            draws: 0,
            total_moves: 0,
            elapsed: started.elapsed(),
            per_game: Vec::with_capacity(self.config.num_games),
        };

        for s in summaries {
            let s = s?;
            result.num_games += 1;
            result.total_moves += u64::from(s.total_moves);
            match s.winner {
                Some(Color::Black) => result.black_wins += 1,
                Some(Color::White) => result.white_wins += 1,
                None => result.draws += 1,
            }
            result.per_game.push(s);
        }
        result.elapsed = started.elapsed();
        Ok(result)
    }

    fn run_one<F>(
        &self,
        index: usize,
        seed: u64,
        factory: &F,
        jsonl_writer: &Option<Mutex<BufWriter<File>>>,
    ) -> Result<GameSummary, BatchError>
    where
        F: Fn(u64) -> (Box<dyn Player>, Box<dyn Player>) + Sync + Send,
    {
        let (p_a, p_b) = factory(seed);
        let colors_swapped = self.config.swap_colors && index % 2 == 1;
        // factory が返した (black, white) を swap 指定時は (white, black) として使う．
        // Player の color() は呼び出し側の Player 実装が固定しているため，BatchRunner 内で
        // ColorAdapter を介して Black/White を強制する．
        let (raw_black, raw_white): (Box<dyn Player>, Box<dyn Player>) = if colors_swapped {
            (p_b, p_a)
        } else {
            (p_a, p_b)
        };

        let mut black_adapter = ColorAdapter {
            inner: raw_black,
            forced: Color::Black,
        };
        let mut white_adapter = ColorAdapter {
            inner: raw_white,
            forced: Color::White,
        };

        let game_id = uuid::Uuid::new_v4().to_string();
        let cfg = EngineConfig {
            board_size: self.config.board_size,
            game_id: Some(game_id.clone()),
            max_moves: self.config.max_moves,
            log_callback: None,
            jsonl_logger: None,
        };
        let mut engine =
            GameEngine::new(cfg).map_err(|e| BatchError::Engine { index, source: e })?;

        let players_meta = PlayerPair {
            black: PlayerInfo::just_name(black_adapter.name().to_string()),
            white: PlayerInfo::just_name(white_adapter.name().to_string()),
        };

        // game_start を JSONL に書く ( 集約ログ)
        let started_at = now();
        if let Some(mtx) = jsonl_writer {
            let event = GameStartEvent {
                ts: started_at,
                game_id: game_id.clone(),
                board_size: [self.config.board_size.rows, self.config.board_size.cols],
                players: PlayerNames {
                    black: players_meta.black.name.clone(),
                    white: players_meta.white.name.clone(),
                },
            };
            write_jsonl_line(mtx, &event)?;
        }

        let result = engine
            .run_with_meta(&mut black_adapter, &mut white_adapter, players_meta.clone())
            .map_err(|e| BatchError::Engine { index, source: e })?;

        // 各手のイベントを JSONL に追記 ( 集約モード)
        if let Some(mtx) = jsonl_writer {
            let history = engine.history();
            let snapshots = history.snapshots();
            for (i, mv) in history.moves().iter().enumerate() {
                let pre = &snapshots[i];
                let post = &snapshots[i + 1];
                let n = (i + 1) as u32;
                let stones = Stones {
                    black: post.board.count(Color::Black),
                    white: post.board.count(Color::White),
                };
                let legal_count = pre.legal_moves().len() as u32;
                let ts = now();
                match mv {
                    othello_core::Move::Place(_) => {
                        let event = MoveEvent {
                            ts,
                            game_id: game_id.clone(),
                            n,
                            side: pre.side_to_move,
                            r#move: (*mv).into(),
                            stones,
                            legal_count,
                        };
                        write_jsonl_line(mtx, &event)?;
                    }
                    othello_core::Move::Pass => {
                        let event = PassEvent {
                            ts,
                            game_id: game_id.clone(),
                            n,
                            side: pre.side_to_move,
                        };
                        write_jsonl_line(mtx, &event)?;
                    }
                }
            }
            // game_end
            let event = GameEndEvent {
                ts: now(),
                game_id: game_id.clone(),
                winner: result.winner,
                stones: Stones {
                    black: result.black,
                    white: result.white,
                },
                moves_total: result.total_moves,
            };
            write_jsonl_line(mtx, &event)?;
        }

        // 各局を JSON で個別保存
        if let Some(dir) = &self.config.log_dir {
            let path = dir.join(format!("game_{:08}.json", index));
            let record = build_record(
                &engine,
                &game_id,
                started_at,
                &players_meta,
                &result,
                &self.config,
            );
            let f = File::create(&path)?;
            let mut buf = BufWriter::new(f);
            JsonWriter::new()
                .write_game(&mut buf, &record)
                .map_err(|e| BatchError::Engine {
                    index,
                    source: EngineError::Logger(e),
                })?;
            buf.flush()?;
        }

        let summary = GameSummary {
            game_id,
            black_score: result.black,
            white_score: result.white,
            winner: result.winner,
            total_moves: result.total_moves,
            colors_swapped,
        };
        // 進捗コールバック
        if let Some(cb) = self.config.progress.as_ref() {
            cb.on_game_complete(index, &summary);
        }
        Ok(summary)
    }
}

fn build_record(
    engine: &GameEngine,
    game_id: &str,
    started_at: DateTime<FixedOffset>,
    players: &PlayerPair,
    result: &GameResult,
    config: &BatchConfig,
) -> GameRecord {
    let metadata = GameMetadata {
        id: game_id.to_string(),
        started_at,
        ended_at: Some(now()),
        board_size: config.board_size,
        players: players.clone(),
        result: Some(GameResultRecord {
            winner: result.winner,
            score: Score {
                black: result.black,
                white: result.white,
            },
        }),
        engine_version: format!("rs-othello-sim {}", env!("CARGO_PKG_VERSION")),
    };
    let history: &GameHistory = engine.history();
    let mut moves: Vec<MoveEntry> = Vec::with_capacity(history.total_moves());
    for (i, (mv, post)) in history
        .moves()
        .iter()
        .zip(history.snapshots().iter().skip(1))
        .enumerate()
    {
        moves.push(MoveEntry {
            n: (i + 1) as u32,
            side: post.side_to_move.opponent(),
            r#move: *mv,
            ts: started_at,
        });
    }
    GameRecord {
        schema_version: SCHEMA_VERSION.to_string(),
        metadata,
        moves,
    }
}

fn write_jsonl_line<T: serde::Serialize>(
    mtx: &Mutex<BufWriter<File>>,
    value: &T,
) -> Result<(), BatchError> {
    let line = serde_json::to_string(value)
        .map_err(|e| BatchError::Io(std::io::Error::other(e.to_string())))?;
    let mut guard = mtx.lock().expect("mutex poisoned");
    guard.write_all(line.as_bytes())?;
    guard.write_all(b"\n")?;
    Ok(())
}

fn now() -> DateTime<FixedOffset> {
    let local = Local::now();
    local.with_timezone(&local.offset().fix())
}

/// `Player` の `color()` を強制的に `Black` または `White` に置換するアダプタ．
///
/// `BatchRunner` で `swap_colors` を扱うために使う．元の Player はゲームの
/// **「黒側の指し手」「白側の指し手」** として使われる．
struct ColorAdapter {
    inner: Box<dyn Player>,
    forced: Color,
}

impl Player for ColorAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn color(&self) -> Color {
        self.forced
    }
    fn select_move(
        &mut self,
        state: &othello_core::GameState,
    ) -> Result<othello_core::Move, PlayerError> {
        self.inner.select_move(state)
    }
    fn on_game_end(&mut self, final_state: &othello_core::GameState, result: GameResult) {
        self.inner.on_game_end(final_state, result)
    }
    fn reset(&mut self) {
        self.inner.reset()
    }
    fn evaluator(&mut self) -> Option<&mut dyn othello_player::Evaluator> {
        self.inner.evaluator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_player::RandomPlayer;

    fn random_factory(seed: u64) -> (Box<dyn Player>, Box<dyn Player>) {
        (
            Box::new(RandomPlayer::with_seed(Color::Black, seed)),
            Box::new(RandomPlayer::with_seed(Color::White, seed.wrapping_add(1))),
        )
    }

    #[test]
    fn runs_random_vs_random_aggregates() {
        let cfg = BatchConfig {
            num_games: 20,
            num_threads: 2,
            seed: Some(7),
            log_dir: None,
            swap_colors: false,
            board_size: BoardSize::STANDARD,
            max_moves: None,
            jsonl_log_path: None,
            progress: None,
        };
        let runner = BatchRunner::new(cfg);
        let result = runner.run(random_factory).unwrap();
        assert_eq!(result.num_games, 20);
        assert_eq!(result.black_wins + result.white_wins + result.draws, 20);
        assert_eq!(result.per_game.len(), 20);
    }

    #[test]
    fn swap_colors_flags_half() {
        let cfg = BatchConfig {
            num_games: 10,
            num_threads: 2,
            seed: Some(3),
            log_dir: None,
            swap_colors: true,
            board_size: BoardSize::STANDARD,
            max_moves: None,
            jsonl_log_path: None,
            progress: None,
        };
        let runner = BatchRunner::new(cfg);
        let result = runner.run(random_factory).unwrap();
        let swapped = result.per_game.iter().filter(|s| s.colors_swapped).count();
        assert_eq!(swapped, 5);
    }

    #[test]
    fn deterministic_with_seed() {
        let cfg = BatchConfig {
            num_games: 8,
            num_threads: 2,
            seed: Some(99),
            log_dir: None,
            swap_colors: false,
            board_size: BoardSize::STANDARD,
            max_moves: None,
            jsonl_log_path: None,
            progress: None,
        };
        let r1 = BatchRunner::new(BatchConfig {
            num_games: cfg.num_games,
            num_threads: cfg.num_threads,
            seed: cfg.seed,
            log_dir: None,
            swap_colors: cfg.swap_colors,
            board_size: cfg.board_size,
            max_moves: cfg.max_moves,
            jsonl_log_path: None,
            progress: None,
        })
        .run(random_factory)
        .unwrap();
        let r2 = BatchRunner::new(BatchConfig {
            num_games: cfg.num_games,
            num_threads: cfg.num_threads,
            seed: cfg.seed,
            log_dir: None,
            swap_colors: cfg.swap_colors,
            board_size: cfg.board_size,
            max_moves: cfg.max_moves,
            jsonl_log_path: None,
            progress: None,
        })
        .run(random_factory)
        .unwrap();
        // 結果セット ( 勝敗・石数・手数) は seed 固定で一致する．
        // ただし game_id は UUIDv4 なので除外比較する．
        assert_eq!(r1.black_wins, r2.black_wins);
        assert_eq!(r1.white_wins, r2.white_wins);
        assert_eq!(r1.draws, r2.draws);
        assert_eq!(r1.total_moves, r2.total_moves);
        let scores1: Vec<_> = r1
            .per_game
            .iter()
            .map(|s| (s.black_score, s.white_score, s.winner, s.total_moves))
            .collect();
        let scores2: Vec<_> = r2
            .per_game
            .iter()
            .map(|s| (s.black_score, s.white_score, s.winner, s.total_moves))
            .collect();
        assert_eq!(scores1, scores2);
    }

    #[test]
    fn log_dir_writes_files() {
        let tmp = std::env::temp_dir().join(format!(
            "rs-othello-sim-batch-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let cfg = BatchConfig {
            num_games: 4,
            num_threads: 2,
            seed: Some(11),
            log_dir: Some(tmp.clone()),
            swap_colors: false,
            board_size: BoardSize::STANDARD,
            max_moves: None,
            jsonl_log_path: None,
            progress: None,
        };
        let runner = BatchRunner::new(cfg);
        let result = runner.run(random_factory).unwrap();
        assert_eq!(result.num_games, 4);
        let entries: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .collect();
        assert_eq!(entries.len(), 4);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
