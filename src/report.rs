//! 결과 집계 타입 — spec §3.2.

use std::path::PathBuf;

/// 하나의 페이지 라스터화 성공 기록.
#[derive(Debug, Clone)]
pub struct PageArtifact {
    /// 원본 문서 경로 (bytes 입력이면 None).
    pub source_path: Option<PathBuf>,
    /// 0-based 페이지 인덱스 (pdfium 원본 체계).
    pub page_index: u32,
    /// 생성된 이미지 파일 경로.
    pub output_path: PathBuf,
    /// 이미지 너비(px).
    pub width_px: u32,
    /// 이미지 높이(px).
    pub height_px: u32,
}

/// 실패한 페이지 또는 문서 수준 실패의 기록.
#[derive(Debug, Clone)]
pub struct PageFailure {
    /// 원본 문서 경로.
    pub source_path: Option<PathBuf>,
    /// 0-based 페이지 인덱스. 문서 수준 실패면 None.
    pub page_index: Option<u32>,
    /// 실패가 일어난 단계.
    pub stage: FailureStage,
    /// 사람이 읽을 수 있는 설명.
    pub message: String,
}

/// 실패가 일어난 파이프라인 단계.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// 소스 파일 읽기.
    SourceRead,
    /// 중간 포맷(`PDF`)으로 변환 (Gotenberg / rhwp).
    Convert,
    /// pdfium 라스터화.
    Rasterize,
    /// 디스크 쓰기.
    Write,
}

impl FailureStage {
    /// `errors.json` 스키마용 소문자-하이픈 표현.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceRead => "source-read",
            Self::Convert => "convert",
            Self::Rasterize => "rasterize",
            Self::Write => "write",
        }
    }
}

/// 배치 처리 결과 집계.
#[derive(Debug, Clone, Default)]
pub struct ExtractReport {
    /// 성공한 페이지 산출물.
    pub succeeded: Vec<PageArtifact>,
    /// 실패 기록.
    pub failed: Vec<PageFailure>,
}

impl ExtractReport {
    /// 새 빈 report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 성공 건수.
    #[must_use]
    pub fn succeeded_count(&self) -> usize {
        self.succeeded.len()
    }

    /// 실패 건수.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    /// 실패가 없으면 true.
    #[must_use]
    pub fn is_fully_successful(&self) -> bool {
        self.failed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_report_is_fully_successful() {
        let r = ExtractReport::new();
        assert!(r.is_fully_successful());
        assert_eq!(r.succeeded_count(), 0);
        assert_eq!(r.failed_count(), 0);
    }

    #[test]
    fn report_with_failure_is_not_fully_successful() {
        let mut r = ExtractReport::new();
        r.failed.push(PageFailure {
            source_path: None,
            page_index: Some(0),
            stage: FailureStage::Rasterize,
            message: "x".to_owned(),
        });
        assert!(!r.is_fully_successful());
        assert_eq!(r.failed_count(), 1);
    }

    #[test]
    fn failure_stage_strings_match_schema() {
        assert_eq!(FailureStage::SourceRead.as_str(), "source-read");
        assert_eq!(FailureStage::Convert.as_str(), "convert");
        assert_eq!(FailureStage::Rasterize.as_str(), "rasterize");
        assert_eq!(FailureStage::Write.as_str(), "write");
    }
}
