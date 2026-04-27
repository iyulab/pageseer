//! `errors.json` 직렬화 — spec §4.4.

use std::path::Path;

use serde::Serialize;

use crate::report::{ExtractReport, PageFailure};

/// `errors.json` 루트 스키마.
#[derive(Debug, Serialize)]
pub struct ErrorsReport {
    /// 스키마 버전 (현재 1).
    pub version: u32,
    /// 실패 항목.
    pub errors: Vec<ErrorEntry>,
}

/// 단일 실패 항목.
#[derive(Debug, Serialize)]
pub struct ErrorEntry {
    /// 원본 문서 경로 (`UTF-8` lossy 변환). 메모리 입력 등 None은 빈 문자열.
    pub source: String,
    /// 1-based 페이지 번호. 문서 수준 실패면 None.
    pub page: Option<u32>,
    /// 실패 단계 (`source-read`, `convert`, `rasterize`, `write`).
    pub stage: &'static str,
    /// 사람이 읽을 수 있는 메시지.
    pub message: String,
}

impl From<&PageFailure> for ErrorEntry {
    fn from(f: &PageFailure) -> Self {
        Self {
            source: f
                .source_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            page: f.page_index.map(|i| i + 1),
            stage: f.stage.as_str(),
            message: f.message.clone(),
        }
    }
}

/// 실패가 0건이면 None — 호출 측은 None 시 파일을 만들지 않는다.
#[must_use]
pub fn build(report: &ExtractReport) -> Option<ErrorsReport> {
    if report.failed.is_empty() {
        return None;
    }
    Some(ErrorsReport {
        version: 1,
        errors: report.failed.iter().map(ErrorEntry::from).collect(),
    })
}

/// `errors.json`을 `dir/errors.json`에 쓴다. 실패 0건이면 아무것도 하지 않고 false 반환.
pub fn write_to_dir(report: &ExtractReport, dir: &Path) -> std::io::Result<bool> {
    let Some(payload) = build(report) else {
        return Ok(false);
    };
    let path = dir.join("errors.json");
    let json = serde_json::to_vec_pretty(&payload).map_err(std::io::Error::other)?;
    std::fs::write(path, json)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{FailureStage, PageFailure};
    use std::path::PathBuf;

    #[test]
    fn empty_report_yields_none() {
        let r = ExtractReport::new();
        assert!(build(&r).is_none());
    }

    #[test]
    fn page_index_serialized_as_one_based() {
        let mut r = ExtractReport::new();
        r.failed.push(PageFailure {
            source_path: Some(PathBuf::from("a.pdf")),
            page_index: Some(0),
            stage: FailureStage::Rasterize,
            message: "boom".into(),
        });
        let payload = build(&r).unwrap();
        assert_eq!(payload.errors[0].page, Some(1));
        assert_eq!(payload.errors[0].stage, "rasterize");
        assert_eq!(payload.errors[0].source, "a.pdf");
    }

    #[test]
    fn document_level_failure_has_no_page() {
        let mut r = ExtractReport::new();
        r.failed.push(PageFailure {
            source_path: None,
            page_index: None,
            stage: FailureStage::SourceRead,
            message: "nope".into(),
        });
        let payload = build(&r).unwrap();
        assert_eq!(payload.errors[0].page, None);
        assert_eq!(payload.errors[0].source, "");
        assert_eq!(payload.errors[0].stage, "source-read");
    }
}
