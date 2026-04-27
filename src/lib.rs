//! pageseer — document-to-page-image rasterizer.
//!
//! See the design spec at `claudedocs/specs/` for architecture.
//!
//! # Status
//!
//! S0 bootstrap: 공개 `API` signature만 노출. 실제 라스터화는 S1+에서 구현.

#![warn(missing_docs)]

use std::path::PathBuf;

pub mod error;
pub mod options;
pub mod raster;
pub mod report;

pub use error::PageseerError;
pub use options::{ImageFormat, Options};
pub use report::{ExtractReport, FailureStage, PageArtifact, PageFailure};

/// 라이브러리 소비자가 넘기는 입력 소스.
#[derive(Debug, Clone)]
pub enum SourceInput {
    /// 파일 경로.
    Path(PathBuf),
    /// 메모리 바이트 + 원본 파일명(포맷 탐지용).
    Bytes {
        /// 문서 바이트.
        data: Vec<u8>,
        /// 포맷 탐지 힌트용 파일명 (`report.docx` 식).
        filename: String,
    },
}

/// 단일 문서를 페이지 이미지로 추출한다.
///
/// # Errors
///
/// - `PageseerError::Partial` — `strict=false` 모드에서 1건 이상 실패.
/// - 그 외 변형 — strict 모드에서 첫 실패 시점에 반환, 혹은 전 입력 실패.
///
/// # Panics
///
/// 현재 버전(S0)은 항상 `PageseerError::Config("not yet implemented (S0 bootstrap)")` 반환.
pub fn extract(_input: SourceInput, _options: Options) -> Result<ExtractReport, PageseerError> {
    Err(PageseerError::Config(
        "not yet implemented (S0 bootstrap)".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_returns_config_error_in_s0() {
        let result = extract(
            SourceInput::Path(PathBuf::from("nonexistent.pdf")),
            Options::default(),
        );
        match result {
            Err(PageseerError::Config(msg)) => {
                assert!(msg.contains("S0"), "message: {msg}");
            }
            other => panic!("expected Config error, got {other:?}"),
        }
    }
}
