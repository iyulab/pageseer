//! 통합 테스트 공용 헬퍼.
//!
//! `cargo test`는 `tests/common/mod.rs`를 별도 test binary로 컴파일하지 않는다.
//! 통합 테스트 파일에서 `mod common;`으로 inline 선언해 사용한다.

#![allow(dead_code)]

use std::path::PathBuf;

/// 격리된 임시 디렉터리. label은 충돌 방지/디버깅용 prefix.
pub fn tempfile_dir(label: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("pageseer-{}-{}", label, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
