//! Integration tests for the `simulate` subcommand.

use std::process::Command;

fn cargo_bin() -> String {
    env!("CARGO_BIN_EXE_othello-cli").to_string()
}

#[test]
fn simulate_creates_json_record() {
    let dir = tempdir_path();
    let out = dir.join("game.json");
    let status = Command::new(cargo_bin())
        .args([
            "simulate",
            "--board-size",
            "8",
            "--black",
            "random:seed=42",
            "--white",
            "greedy",
            "--save-record",
            out.to_str().unwrap(),
            "--record-format",
            "json",
        ])
        .status()
        .expect("failed to spawn cli");
    assert!(status.success(), "simulate exited non-zero");
    assert!(out.exists(), "record file was not created");
    let contents = std::fs::read_to_string(&out).unwrap();
    assert!(contents.contains("\"schema_version\""));
    assert!(contents.contains("\"moves\""));
}

#[test]
fn simulate_creates_ggf_record() {
    let dir = tempdir_path();
    let out = dir.join("game.ggf");
    let status = Command::new(cargo_bin())
        .args([
            "simulate",
            "--board-size",
            "8",
            "--black",
            "random:seed=1",
            "--white",
            "random:seed=2",
            "--save-record",
            out.to_str().unwrap(),
            "--record-format",
            "ggf",
        ])
        .status()
        .expect("failed to spawn cli");
    assert!(status.success());
    let contents = std::fs::read_to_string(&out).unwrap();
    assert!(contents.starts_with("(;GM[Othello]"));
}

fn tempdir_path() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    let suffix: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    p.push(format!("rs-othello-sim-test-{suffix}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
