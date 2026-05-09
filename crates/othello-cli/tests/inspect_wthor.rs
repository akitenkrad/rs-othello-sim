//! `inspect` サブコマンドの WTHOR ブランチに対する統合テスト．
//!
//! 自前で生成した小さな WTHOR ファイルを読み込み，期待される統計が出力されることを確認する．

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// 8x8 用の最小限ヘッダを作る．
fn make_header(n_games: u32, year: u16) -> [u8; 16] {
    let mut h = [0u8; 16];
    h[4..8].copy_from_slice(&n_games.to_le_bytes());
    h[8..10].copy_from_slice(&year.to_le_bytes());
    h[10] = 8; // board size
    h[11] = 0; // Othello
    h
}

/// `(row, col)` を WTHOR move コードに変換する．
fn encode_move_byte(row: u8, col: u8) -> u8 {
    (row + 1) * 10 + (col + 1)
}

fn make_game_block(real_score: u8, theoretical: u8, moves: &[(u8, u8)]) -> [u8; 68] {
    let mut b = [0u8; 68];
    b[0..2].copy_from_slice(&0u16.to_le_bytes());
    b[2..4].copy_from_slice(&100u16.to_le_bytes());
    b[4..6].copy_from_slice(&200u16.to_le_bytes());
    b[6] = real_score;
    b[7] = theoretical;
    for (i, (r, c)) in moves.iter().enumerate() {
        if i >= 60 {
            break;
        }
        b[8 + i] = encode_move_byte(*r, *c);
    }
    b
}

fn write_wthor(path: &PathBuf, n_games: u32, blocks: &[[u8; 68]]) {
    let mut f = File::create(path).unwrap();
    f.write_all(&make_header(n_games, 2023)).unwrap();
    for b in blocks {
        f.write_all(b).unwrap();
    }
}

fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rs-othello-sim-inspect-wthor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn inspect_wthor_reports_aggregated_stats() {
    // 3 局: 黒勝 / 白勝 / 引分
    let blocks = vec![
        make_game_block(40, 40, &[(2, 3), (4, 2)]), // black 40, white 24 → black wins
        make_game_block(20, 20, &[(2, 3)]),         // black 20, white 44 → white wins
        make_game_block(32, 32, &[(2, 3)]),         // black 32, white 32 → draw
    ];
    let path = temp_path("test.wtb");
    write_wthor(&path, 3, &blocks);

    // CLI バイナリを直接呼ばずに inspect の core 関数を呼ぶには pub 公開する必要があるが，
    // ここでは end-to-end として cargo run で呼ぶ ( CI 環境でも動くようにシンプルに)．
    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let output = std::process::Command::new(exe)
        .arg("inspect")
        .arg("--file")
        .arg(&path)
        .arg("--format")
        .arg("wthor")
        .output()
        .expect("failed to invoke othello-cli");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        output.status.success(),
        "inspect failed: stdout={stdout} stderr={stderr}"
    );

    // 期待される行
    assert!(stdout.contains("Format:      wthor"), "stdout = {stdout}");
    assert!(stdout.contains("Games:       3"));
    assert!(stdout.contains("Year:        2023"));
    assert!(stdout.contains("Board:       8x8"));
    // 勝敗は 1/3 ずつ → 33.3% それぞれ
    assert!(stdout.contains("Black wins:  33.3%"), "stdout = {stdout}");
    assert!(stdout.contains("White wins: 33.3%"));
    assert!(stdout.contains("Draws: 33.3%"));

    // クリーンアップ
    if let Some(parent) = path.parent() {
        std::fs::remove_dir_all(parent).ok();
    }
}

#[test]
fn inspect_wthor_handles_empty_file() {
    let blocks: Vec<[u8; 68]> = vec![];
    let path = temp_path("empty.wtb");
    write_wthor(&path, 0, &blocks);

    let exe = env!("CARGO_BIN_EXE_othello-cli");
    let output = std::process::Command::new(exe)
        .arg("inspect")
        .arg("--file")
        .arg(&path)
        .arg("--format")
        .arg("wthor")
        .output()
        .expect("failed to invoke othello-cli");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(output.status.success(), "stdout = {stdout}");
    assert!(stdout.contains("Games:       0"));

    if let Some(parent) = path.parent() {
        std::fs::remove_dir_all(parent).ok();
    }
}
