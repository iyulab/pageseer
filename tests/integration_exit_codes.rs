//! `CLI` 종료 코드 검증 — spec §4.5.

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pageseer")
}

#[test]
fn unsupported_format_exits_64() {
    let status = Command::new(bin())
        .arg("foo.xyz")
        .status()
        .expect("failed to run pageseer");
    assert_eq!(status.code(), Some(64));
}

#[test]
fn unknown_format_flag_exits_64() {
    let status = Command::new(bin())
        .arg("foo.pdf")
        .args(["-f", "webp"])
        .status()
        .expect("failed to run pageseer");
    assert_eq!(status.code(), Some(64));
}

#[test]
fn missing_required_input_exits_with_clap_error() {
    let status = Command::new(bin())
        .status()
        .expect("failed to run pageseer");
    // clap의 missing-required는 exit code 2 (clap default), spec §4.5 64 카테고리는 아님.
    // pageseer 자체 분기에 들어가지도 않으므로 clap 동작을 그대로 검증.
    assert_eq!(status.code(), Some(2));
}

#[test]
fn help_includes_s5_options() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to run pageseer --help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in [
        "--quality",
        "--max-edge",
        "--flat",
        "--concurrency",
        "--strict",
        "--gotenberg-url",
    ] {
        assert!(
            stdout.contains(needle),
            "--help missing flag {needle}; stdout:\n{stdout}"
        );
    }
}
