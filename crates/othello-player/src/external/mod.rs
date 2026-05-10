//! Player implementation that talks to an external engine process
//! (Edax, Egaroucid, etc.).
//!
//! Implements §3.2.2 of the design document, completed in Phase 5.
//!
//! - Supports [`Protocol::Gtp`] (default) and [`Protocol::Ntest`].
//! - Uses synchronous I/O (`std::process::Command` + `BufReader`); no
//!   `tokio` dependency.
//! - The per-move timeout is implemented softly with a separate thread
//!   plus a channel.
//! - `Drop` sends `quit` to the engine and then kills the process.
//!
//! ## Example
//!
//! ```ignore
//! use othello_player::{ExternalEngineConfig, ExternalEnginePlayer, Protocol, Player};
//! use othello_core::{BoardSize, Color, GameState};
//! use std::path::PathBuf;
//! use std::time::Duration;
//!
//! let cfg = ExternalEngineConfig {
//!     command: PathBuf::from("./mock_engine_gtp.sh"),
//!     args: vec![],
//!     protocol: Protocol::Gtp,
//!     board_size: BoardSize::STANDARD,
//!     timeout: Duration::from_secs(10),
//!     working_dir: None,
//!     env: vec![],
//! };
//! let mut p = ExternalEnginePlayer::new(Color::Black, cfg);
//! let s = GameState::standard_8x8();
//! let _mv = p.select_move(&s).unwrap();
//! ```

pub mod gtp;
pub mod ntest;
pub mod protocol;

use crate::traits::{Player, PlayerError};
use othello_core::{BoardSize, Color, GameResult, GameState, Move};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub use gtp::GtpProtocol;
pub use ntest::NtestProtocol;
pub use protocol::{EngineProtocol, Protocol};

/// Configuration for the external-engine player.
#[derive(Debug, Clone)]
pub struct ExternalEngineConfig {
    /// Path to the executable.
    pub command: PathBuf,
    /// Launch arguments.
    pub args: Vec<String>,
    /// Communication protocol.
    pub protocol: Protocol,
    /// Board size (announced via `boardsize` etc.).
    pub board_size: BoardSize,
    /// Per-move timeout (default 30 seconds).
    pub timeout: Duration,
    /// Working directory to launch the process in.
    pub working_dir: Option<PathBuf>,
    /// Additional environment variables.
    pub env: Vec<(String, String)>,
}

impl ExternalEngineConfig {
    /// Builds a config from the required fields, defaulting the rest.
    #[must_use]
    pub fn new(command: PathBuf, protocol: Protocol, board_size: BoardSize) -> Self {
        Self {
            command,
            args: Vec::new(),
            protocol,
            board_size,
            timeout: Duration::from_secs(30),
            working_dir: None,
            env: Vec::new(),
        }
    }
}

/// External-engine player.
///
/// The process is spawned lazily on the first call to `select_move`.
/// Calling `reset` re-issues `clear_board`, allowing the same process to
/// be reused for subsequent games. `Drop` sends `quit` and then kills
/// the process.
pub struct ExternalEnginePlayer {
    config: ExternalEngineConfig,
    color: Color,
    name: String,
    process: Option<EngineProcess>,
    /// Last move number at which the engine received a `play`
    /// notification (used to keep history in sync).
    last_known_move_number: u32,
}

/// Spawned process plus its IO handles.
struct EngineProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    protocol: Box<dyn EngineProtocol + Send>,
    started: bool,
}

impl ExternalEnginePlayer {
    /// Builds a player from the given configuration. The process is not
    /// started yet.
    #[must_use]
    pub fn new(color: Color, config: ExternalEngineConfig) -> Self {
        let name = format!(
            "ExternalEnginePlayer({}:{})",
            config
                .command
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?"),
            config.protocol.as_str()
        );
        Self {
            config,
            color,
            name,
            process: None,
            last_known_move_number: 0,
        }
    }

    /// Builder: changes the display name.
    #[must_use]
    pub fn name_with(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &ExternalEngineConfig {
        &self.config
    }

    fn ensure_process(&mut self) -> Result<(), PlayerError> {
        if self.process.is_some() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(dir) = &self.config.working_dir {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.config.env {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| PlayerError::Other(format!("failed to spawn engine: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PlayerError::Other("missing stdin handle".into()))?;
        let stdout_raw = child
            .stdout
            .take()
            .ok_or_else(|| PlayerError::Other("missing stdout handle".into()))?;
        let stdout = BufReader::new(stdout_raw);
        let protocol: Box<dyn EngineProtocol + Send> = match self.config.protocol {
            Protocol::Gtp => Box::new(GtpProtocol::new()),
            Protocol::Ntest => Box::new(NtestProtocol::new()),
        };
        self.process = Some(EngineProcess {
            child,
            stdin,
            stdout,
            protocol,
            started: false,
        });
        Ok(())
    }

    /// Shuts down the process (sends `quit`, then kills it). Errors are
    /// ignored.
    fn shutdown(&mut self) {
        if let Some(mut proc) = self.process.take() {
            let _ = proc.protocol.quit(&mut proc.stdin);
            // best-effort kill
            let _ = proc.child.kill();
            let _ = proc.child.wait();
        }
    }
}

impl Drop for ExternalEnginePlayer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Player for ExternalEnginePlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color {
        self.color
    }

    fn select_move(&mut self, state: &GameState) -> Result<Move, PlayerError> {
        self.ensure_process()?;
        let timeout = self.config.timeout;
        let board_size = self.config.board_size;
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| PlayerError::Other("engine process unavailable".into()))?;

        // 初回ならゲーム開始処理 ( clear_board / boardsize)．
        if !process.started {
            run_with_timeout(timeout, || {
                process.protocol.start_game(
                    &mut process.stdin,
                    &mut process.stdout,
                    board_size,
                    timeout,
                )
            })?;
            process.started = true;
            self.last_known_move_number = 0;
        }

        // history 同期: state.move_number まで進んでいない手を play で通知する．
        // ただし state には盤面のみで履歴は含まれないので，現状は「最後に同期した手数までは
        // notify 済み」と仮定し，現局面が新規で進んでいても 1 手分だけのキャッチアップに留める．
        // engine は GTP の `play` で相手手を必要とするが，我々は履歴を持たないため，
        // 「engine 側で相手 ( = 自分以外の手) を 1 手前に play 通知する」というキャッチアップは
        // 呼び出し側 ( on_opponent_move 等) からの責務に近い．
        //
        // シンプル化のため: 直前手 ( state.last_move) があれば play 通知する．
        if let Some(last) = state.last_move
            && state.move_number > self.last_known_move_number
        {
            // 直前手は state.side_to_move の相手側が打った
            let mover = state.side_to_move.opponent();
            run_with_timeout(timeout, || {
                process.protocol.notify_move(
                    &mut process.stdin,
                    &mut process.stdout,
                    mover,
                    last,
                    timeout,
                )
            })?;
            self.last_known_move_number = state.move_number;
        }

        let mv = run_with_timeout(timeout, || {
            process.protocol.request_move(
                &mut process.stdin,
                &mut process.stdout,
                self.color,
                state,
                timeout,
            )
        })?;

        // 自分の手も engine 側に play 通知する ( 次の同期のため)
        run_with_timeout(timeout, || {
            process.protocol.notify_move(
                &mut process.stdin,
                &mut process.stdout,
                self.color,
                mv,
                timeout,
            )
        })?;
        // engine 視点では自分の手を打った直後 = state.move_number + 1
        self.last_known_move_number = state.move_number.saturating_add(1);

        Ok(mv)
    }

    fn on_game_end(&mut self, _final_state: &GameState, _result: GameResult) {
        // engine 終了通知 ( quit) はしない．次ゲームに reset で備える．
    }

    fn reset(&mut self) {
        // プロセスは継続使用．次の `select_move` で `clear_board` を再送するため
        // started フラグを倒す．
        if let Some(p) = self.process.as_mut() {
            p.started = false;
        }
        self.last_known_move_number = 0;
    }
}

/// Runs a closure with a timeout (uses a separate thread plus a channel).
///
/// Returns `Ok(T)` if the closure returns `T`, or `PlayerError::Other` on
/// timeout. Note: on timeout the worker thread keeps running in the
/// background (blocked on IO). Killing the process will cause subsequent
/// IO to hit EOF, allowing the thread to finish eventually.
fn run_with_timeout<F, T>(timeout: Duration, f: F) -> Result<T, PlayerError>
where
    F: FnOnce() -> Result<T, PlayerError> + Send,
    T: Send,
{
    // `f` をライフタイム制約のある参照渡しでクロージャ的に扱うため，scoped thread を使う．
    let (tx, rx) = mpsc::channel::<Result<T, PlayerError>>();
    thread::scope(|scope| {
        let handle = scope.spawn(move || {
            let r = f();
            let _ = tx.send(r);
        });
        let recv_result = rx.recv_timeout(timeout);
        match recv_result {
            Ok(r) => {
                // ワーカーは正常終了済み (channel 送信後)
                let _ = handle.join();
                r
            }
            Err(_) => {
                // タイムアウト．ワーカーは IO で blocked の可能性．
                // process kill 後にスレッドは EOF で抜けるのを期待し，ここでは継続させる．
                Err(PlayerError::Other(format!(
                    "engine response timed out after {timeout:?}"
                )))
            }
        }
    })
}

// 簡易ユーティリティ: BufRead を実装した型は `&mut dyn BufRead` で渡せる．
// trait object 用に explicit impl を追加する必要はない．
#[allow(dead_code)]
fn _assert_buf_read<R: BufRead>(_r: &mut R) {}

#[allow(dead_code)]
fn _assert_write<W: Write>(_w: &mut W) {}

#[cfg(test)]
mod tests {
    use super::*;
    use othello_core::BoardSize;

    #[test]
    fn config_new_defaults() {
        let cfg = ExternalEngineConfig::new(
            PathBuf::from("/bin/false"),
            Protocol::Gtp,
            BoardSize::STANDARD,
        );
        assert_eq!(cfg.timeout, Duration::from_secs(30));
        assert!(cfg.args.is_empty());
        assert_eq!(cfg.protocol, Protocol::Gtp);
    }

    #[test]
    fn name_with_overrides() {
        let cfg = ExternalEngineConfig::new(
            PathBuf::from("/bin/false"),
            Protocol::Gtp,
            BoardSize::STANDARD,
        );
        let p = ExternalEnginePlayer::new(Color::Black, cfg).name_with("MyEngine");
        assert_eq!(p.name(), "MyEngine");
    }
}
