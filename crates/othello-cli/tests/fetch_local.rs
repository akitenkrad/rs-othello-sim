//! Integration tests for the `fetch wthor` subcommand.
//!
//! To avoid real network access, a local `tiny_http` server is started
//! and the flow is verified by pointing `--url-pattern` at it.

use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tiny_http::{Method, Response, Server};
use zip::write::SimpleFileOptions;

/// Creates a uniquely named temporary directory for the tests.
fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rs-othello-sim-fetch-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Builds a minimal WTHOR byte sequence (a 16-byte header plus one
/// 68-byte game).
fn make_wthor_bytes() -> Vec<u8> {
    let mut header = [0u8; 16];
    header[4..8].copy_from_slice(&1u32.to_le_bytes()); // n_games
    header[8..10].copy_from_slice(&2023u16.to_le_bytes()); // year
    header[10] = 8; // board size
    header[11] = 0; // Othello

    let mut block = [0u8; 68];
    block[0..2].copy_from_slice(&0u16.to_le_bytes());
    block[2..4].copy_from_slice(&100u16.to_le_bytes());
    block[4..6].copy_from_slice(&200u16.to_le_bytes());
    block[6] = 32;
    block[7] = 32;
    block[8] = 34; // D3 (2,3) → 3*10 + 4

    let mut v = Vec::new();
    v.extend_from_slice(&header);
    v.extend_from_slice(&block);
    v
}

/// Builds an in-memory zip archive containing a single `wth_2023.wtb`
/// entry.
fn make_wtb_zip(file_name: &str) -> Vec<u8> {
    let buf = Cursor::new(Vec::<u8>::new());
    let mut zw = zip::ZipWriter::new(buf);
    let opts: SimpleFileOptions =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zw.start_file(file_name, opts).unwrap();
    zw.write_all(&make_wthor_bytes()).unwrap();
    let mut cursor = zw.finish().unwrap();
    cursor.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    cursor.read_to_end(&mut out).unwrap();
    out
}

/// Starts a simple HTTP server that returns a single file and yields
/// `(port, shutdown_handle)`.
struct LocalServer {
    port: u16,
    shutdown: mpsc::Sender<()>,
    handle: Option<thread::JoinHandle<()>>,
}

impl LocalServer {
    fn start_static(payload: Vec<u8>) -> Self {
        let server = Server::http("127.0.0.1:0").expect("failed to bind local HTTP server");
        let port = server.server_addr().to_ip().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();
        let handle = thread::spawn(move || {
            loop {
                if rx.try_recv().is_ok() {
                    break;
                }
                match server.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(req)) => {
                        if req.method() == &Method::Get {
                            let resp = Response::from_data(payload.clone());
                            let _ = req.respond(resp);
                        } else {
                            let _ = req.respond(Response::empty(405));
                        }
                    }
                    Ok(None) => continue,
                    Err(_) => break,
                }
            }
        });
        LocalServer {
            port,
            shutdown: tx,
            handle: Some(handle),
        }
    }

    fn start_404() -> Self {
        let server = Server::http("127.0.0.1:0").expect("failed to bind local HTTP server");
        let port = server.server_addr().to_ip().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();
        let handle = thread::spawn(move || {
            loop {
                if rx.try_recv().is_ok() {
                    break;
                }
                match server.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(req)) => {
                        let _ = req.respond(Response::empty(404));
                    }
                    Ok(None) => continue,
                    Err(_) => break,
                }
            }
        });
        LocalServer {
            port,
            shutdown: tx,
            handle: Some(handle),
        }
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[test]
fn fetch_wthor_downloads_and_extracts_local_zip() {
    let dest = temp_dir("dest");
    let payload = make_wtb_zip("wth_2023.wtb");
    let server = LocalServer::start_static(payload);

    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let url_pattern = format!("http://127.0.0.1:{}/wth_{{YEAR}}.zip", server.port);

    // 1) ダウンロード→展開
    let output = std::process::Command::new(exe)
        .arg("fetch")
        .arg("wthor")
        .arg("--year")
        .arg("2023")
        .arg("--url-pattern")
        .arg(&url_pattern)
        .arg("--dest")
        .arg(&dest)
        .arg("--no-progress")
        .output()
        .expect("failed to invoke othello-cli");
    assert!(
        output.status.success(),
        "fetch failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let wtb = dest.join("wth_2023.wtb");
    assert!(wtb.exists(), "expected {} to exist", wtb.display());
    let zip_path = dest.join("wth_2023.zip");
    assert!(
        !zip_path.exists(),
        "archive zip should be removed without --keep-archive"
    );

    // 内容が WTHOR として読めることを確認 ( 軽い sanity check)
    let file = fs::File::open(&wtb).unwrap();
    let mut reader =
        othello_io::WthorReader::new(std::io::BufReader::new(file)).expect("WthorReader");
    let games = reader.read_all().expect("WTHOR read_all");
    assert_eq!(games.len(), 1);

    // 2) もう一度実行 → 既存ファイルなのでスキップされる
    let output2 = std::process::Command::new(exe)
        .arg("fetch")
        .arg("wthor")
        .arg("--year")
        .arg("2023")
        .arg("--url-pattern")
        .arg(&url_pattern)
        .arg("--dest")
        .arg(&dest)
        .arg("--no-progress")
        .output()
        .expect("failed to invoke othello-cli");
    assert!(output2.status.success());
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr2.contains("already exists"),
        "expected 'already exists' message; stderr = {stderr2}"
    );

    fs::remove_dir_all(&dest).ok();
}

#[test]
fn fetch_wthor_keep_archive_preserves_zip() {
    let dest = temp_dir("keep");
    let payload = make_wtb_zip("wth_2024.wtb");
    let server = LocalServer::start_static(payload);

    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let url_pattern = format!("http://127.0.0.1:{}/wth_{{YEAR}}.zip", server.port);

    let output = std::process::Command::new(exe)
        .arg("fetch")
        .arg("wthor")
        .arg("--year")
        .arg("2024")
        .arg("--url-pattern")
        .arg(&url_pattern)
        .arg("--dest")
        .arg(&dest)
        .arg("--keep-archive")
        .arg("--no-progress")
        .output()
        .expect("failed to invoke othello-cli");
    assert!(
        output.status.success(),
        "fetch failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let zip_path = dest.join("wth_2024.zip");
    assert!(zip_path.exists(), "zip should be kept with --keep-archive");
    let wtb_path = dest.join("wth_2024.wtb");
    assert!(wtb_path.exists());

    fs::remove_dir_all(&dest).ok();
}

#[test]
fn fetch_wthor_404_returns_friendly_error() {
    let dest = temp_dir("404");
    let server = LocalServer::start_404();

    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let url_pattern = format!("http://127.0.0.1:{}/wth_{{YEAR}}.zip", server.port);

    let output = std::process::Command::new(exe)
        .arg("fetch")
        .arg("wthor")
        .arg("--year")
        .arg("2099")
        .arg("--url-pattern")
        .arg(&url_pattern)
        .arg("--dest")
        .arg(&dest)
        .arg("--no-progress")
        .output()
        .expect("failed to invoke othello-cli");
    assert!(
        !output.status.success(),
        "expected failure for 404 response"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("404"),
        "expected 404 in stderr; got: {stderr}"
    );
    assert!(
        stderr.contains("ffothello.org"),
        "expected guidance to ffothello.org; got: {stderr}"
    );
    assert!(
        stderr.contains("--url-pattern") || stderr.contains("OTHELLO_WTHOR_URL_PATTERN"),
        "expected mention of url-pattern override; got: {stderr}"
    );

    fs::remove_dir_all(&dest).ok();
}

#[test]
fn fetch_list_lists_wthor() {
    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let output = std::process::Command::new(exe)
        .arg("fetch")
        .arg("list")
        .output()
        .expect("failed to invoke othello-cli");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wthor"));
    assert!(stdout.contains("French Othello Federation"));
    assert!(stdout.contains("ffothello.org"));
    assert!(stdout.contains("docs/external-data.md"));
}

#[test]
#[ignore = "requires network access to ffothello.org"]
fn fetch_real_wthor_2023() {
    // 実機テスト: ネットワーク必須なので既定では走らない．
    // `cargo test -p othello-cli --test fetch_local -- --ignored` で実行．
    let dest = temp_dir("real");
    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let output = std::process::Command::new(exe)
        .arg("fetch")
        .arg("wthor")
        .arg("--year")
        .arg("2023")
        .arg("--dest")
        .arg(&dest)
        .arg("--no-progress")
        .output()
        .expect("failed to invoke othello-cli");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        let any_wtb = fs::read_dir(&dest)
            .map(|it| {
                it.filter_map(|e| e.ok()).any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(".wtb")
                })
            })
            .unwrap_or(false);
        assert!(
            any_wtb,
            "expected at least one .wtb file in {}, stderr={}",
            dest.display(),
            stderr
        );
    } else {
        eprintln!(
            "real-network fetch did not succeed (this is allowed): {}",
            stderr
        );
    }

    fs::remove_dir_all(&dest).ok();
}
