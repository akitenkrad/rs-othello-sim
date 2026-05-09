//! 外部プロセスエンジン ( Edax / Egaroucid 等) と連携するプレイヤー実装．
//!
//! 設計書 §3.2.2 を Phase 5 で実装したもの．
//!
//! - [`Protocol::Gtp`] ( デフォルト) と [`Protocol::Ntest`] の 2 種類をサポート
//! - 同期 IO ( `std::process::Command` + `BufReader`) で実装し，`tokio` 等の依存は持たない
//! - 1 手あたりタイムアウトは別スレッド + channel でソフトに実装
//! - `Drop` でプロセスを `quit` → kill する
//!
//! ## 例
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

/// 外部エンジンプレイヤーの設定．
#[derive(Debug, Clone)]
pub struct ExternalEngineConfig {
    /// 実行ファイルパス．
    pub command: PathBuf,
    /// 起動時引数．
    pub args: Vec<String>,
    /// 通信プロトコル．
    pub protocol: Protocol,
    /// 盤面サイズ ( `boardsize` 等で通知)．
    pub board_size: BoardSize,
    /// 1 手あたりタイムアウト ( デフォルト 30 秒)．
    pub timeout: Duration,
    /// 起動時のカレントディレクトリ．
    pub working_dir: Option<PathBuf>,
    /// 追加環境変数．
    pub env: Vec<(String, String)>,
}

impl ExternalEngineConfig {
    /// 必須項目のみで設定を構築する ( 他はデフォルト)．
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

/// 外部エンジンプレイヤー．
///
/// プロセスは `select_move` 初回呼び出し時に起動される ( 遅延起動)．
/// `reset` で `clear_board` を再送出してゲーム継続使用が可能．
/// `Drop` で `quit` 送信 + プロセス kill．
pub struct ExternalEnginePlayer {
    config: ExternalEngineConfig,
    color: Color,
    name: String,
    process: Option<EngineProcess>,
    /// engine が `play` 通知を受けた最後の手数 ( history 同期用)．
    last_known_move_number: u32,
}

/// 起動済みプロセス + IO ハンドル．
struct EngineProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    protocol: Box<dyn EngineProtocol + Send>,
    started: bool,
}

impl ExternalEnginePlayer {
    /// 設定からプレイヤーを生成する．プロセスは未起動．
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

    /// 表示名を変更する ( builder)．
    #[must_use]
    pub fn name_with(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 設定参照．
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

    /// プロセスを終了する ( `quit` 送信 + kill)．エラーは無視．
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

/// クロージャをタイムアウト付きで実行する ( 別スレッド + channel)．
///
/// クロージャが `T` を返したら `Ok(T)`，タイムアウトしたら `PlayerError::Other`．
/// 注意: タイムアウト時はワーカースレッドはバックグラウンドで継続する ( IO 待ちで止まる)．
/// プロセス kill により後続 IO は EOF になるので，最終的にスレッドは終了する．
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
