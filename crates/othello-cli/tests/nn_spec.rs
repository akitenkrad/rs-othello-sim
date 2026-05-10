//! `nn:` PlayerSpec パース + CLI からのエラーメッセージ検証 ( Phase 6.4)．

use othello_player::{NnBackend, PlayerSpec, parse_player_spec};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn parses_nn_safetensors_basic() {
    let s = parse_player_spec("nn:safetensors:/tmp/dummy.safetensors").unwrap();
    match s {
        PlayerSpec::Nn(spec) => {
            assert_eq!(spec.backend, NnBackend::Safetensors);
            assert_eq!(spec.path, PathBuf::from("/tmp/dummy.safetensors"));
        }
        _ => panic!("expected Nn"),
    }
}

#[test]
fn parses_nn_onnx_with_options() {
    let s =
        parse_player_spec("nn:onnx:./policy.onnx,temperature=0.5,deterministic,seed=42").unwrap();
    match s {
        PlayerSpec::Nn(spec) => {
            assert_eq!(spec.backend, NnBackend::Onnx);
            assert_eq!(spec.temperature, Some(0.5));
            assert!(spec.deterministic);
            assert_eq!(spec.seed, Some(42));
        }
        _ => panic!("expected Nn"),
    }
}

/// `nn:safetensors:<missing-file>` を CLI に渡すと non-zero exit + エラーメッセージ．
#[test]
fn cli_simulate_with_missing_safetensors_errors() {
    let bin = env!("CARGO_BIN_EXE_othello-cli");
    let output = Command::new(bin)
        .args([
            "simulate",
            "--black",
            "nn:safetensors:/tmp/__never_exists__.safetensors",
            "--white",
            "greedy",
        ])
        .output()
        .expect("run othello-cli");
    assert!(
        !output.status.success(),
        "expected non-zero exit, got status {}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("safetensors")
            || combined.contains("not found")
            || combined.contains("failed"),
        "expected error mention safetensors/not found/failed, got: {combined}"
    );
}
