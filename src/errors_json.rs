//! `errors.json` 직렬화 — spec §4.4.

use std::path::Path;

use serde::Serialize;

use crate::report::{
    BatchReport, BatchSummary, DocumentOutcome, DocumentResult, ExtractReport, PageFailure,
};

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

/// `errors.json` v2 루트 스키마.
#[derive(Debug, Serialize)]
pub struct ErrorsReportV2 {
    /// 스키마 버전 (현재 2).
    pub version: u32,
    /// 배치 카운트 요약.
    pub summary: BatchSummary,
    /// 실패 항목.
    pub errors: Vec<ErrorEntryV2>,
}

/// v2 단일 실패 항목.
#[derive(Debug, Serialize)]
pub struct ErrorEntryV2 {
    /// 입력 인덱스.
    pub document_index: u32,
    /// 원본 라벨 (Path 입력은 lossy 경로, Bytes는 filename).
    pub source: String,
    /// 1-based 페이지 번호. 문서 수준 실패면 None.
    pub page: Option<u32>,
    /// 실패 단계.
    pub stage: &'static str,
    /// 사람이 읽을 수 있는 메시지.
    pub message: String,
}

impl serde::Serialize for BatchSummary {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = ser.serialize_struct("BatchSummary", 7)?;
        s.serialize_field("documents_total", &self.documents_total)?;
        s.serialize_field("documents_succeeded", &self.documents_succeeded)?;
        s.serialize_field("documents_partial", &self.documents_partial)?;
        s.serialize_field("documents_failed", &self.documents_failed)?;
        s.serialize_field("documents_skipped", &self.documents_skipped)?;
        s.serialize_field("pages_succeeded", &self.pages_succeeded)?;
        s.serialize_field("pages_failed", &self.pages_failed)?;
        s.end()
    }
}

/// 실패가 0건이면 None — 호출 측은 None 시 파일을 만들지 않는다.
#[must_use]
pub fn build_v2(report: &BatchReport) -> Option<ErrorsReportV2> {
    let mut errors: Vec<ErrorEntryV2> = Vec::new();
    for doc in &report.documents {
        match &doc.outcome {
            DocumentOutcome::Processed(inner) => {
                for f in &inner.failed {
                    errors.push(entry_v2_for(doc, f));
                }
            }
            DocumentOutcome::Failed(f) => {
                errors.push(entry_v2_for(doc, f));
            }
            DocumentOutcome::Skipped => {}
        }
    }
    if errors.is_empty() {
        return None;
    }
    Some(ErrorsReportV2 {
        version: 2,
        summary: report.summary,
        errors,
    })
}

fn entry_v2_for(doc: &DocumentResult, f: &PageFailure) -> ErrorEntryV2 {
    ErrorEntryV2 {
        document_index: doc.input_index,
        source: f.source_path.as_ref().map_or_else(
            || doc.source_label.clone(),
            |p| p.to_string_lossy().into_owned(),
        ),
        page: f.page_index.map(|i| i + 1),
        stage: f.stage.as_str(),
        message: f.message.clone(),
    }
}

/// `<dir>/errors.json` (v2) 작성. 실패 0건이면 false 반환.
pub fn write_v2_to_dir(report: &BatchReport, dir: &Path) -> std::io::Result<bool> {
    let Some(payload) = build_v2(report) else {
        return Ok(false);
    };
    let path = dir.join("errors.json");
    let json = serde_json::to_vec_pretty(&payload).map_err(std::io::Error::other)?;
    std::fs::write(path, json)?;
    Ok(true)
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

    #[test]
    fn v2_includes_summary_and_document_index() {
        use crate::report::{
            BatchReport, BatchSummary, DocumentOutcome, DocumentResult, ExtractReport,
        };
        use std::path::PathBuf;

        let mut inner = ExtractReport::new();
        inner.failed.push(PageFailure {
            source_path: Some(PathBuf::from("a.pdf")),
            page_index: Some(2),
            stage: FailureStage::Rasterize,
            message: "rasterize boom".into(),
        });
        let docs = vec![DocumentResult {
            input_index: 0,
            source_label: "a.pdf".into(),
            output_dir: PathBuf::from("./out/a"),
            outcome: DocumentOutcome::Processed(inner),
        }];
        let summary = BatchSummary::from_documents(&docs);
        let report = BatchReport {
            documents: docs,
            summary,
        };
        let payload = build_v2(&report).expect("expected non-empty payload");
        assert_eq!(payload.version, 2);
        assert_eq!(payload.summary.documents_total, 1);
        assert_eq!(payload.summary.pages_failed, 1);
        assert_eq!(payload.errors.len(), 1);
        assert_eq!(payload.errors[0].document_index, 0);
        assert_eq!(payload.errors[0].source, "a.pdf");
        assert_eq!(payload.errors[0].page, Some(3));
        assert_eq!(payload.errors[0].stage, "rasterize");
    }

    #[test]
    fn v2_document_failed_appears_with_null_page() {
        use crate::report::{BatchReport, BatchSummary, DocumentOutcome, DocumentResult};
        use std::path::PathBuf;

        let docs = vec![DocumentResult {
            input_index: 1,
            source_label: "b.pdf".into(),
            output_dir: PathBuf::from("./out/b"),
            outcome: DocumentOutcome::Failed(PageFailure {
                source_path: Some(PathBuf::from("b.pdf")),
                page_index: None,
                stage: FailureStage::Convert,
                message: "corrupt".into(),
            }),
        }];
        let report = BatchReport {
            summary: BatchSummary::from_documents(&docs),
            documents: docs,
        };
        let payload = build_v2(&report).expect("expected non-empty payload");
        assert_eq!(payload.errors[0].document_index, 1);
        assert_eq!(payload.errors[0].page, None);
        assert_eq!(payload.errors[0].stage, "convert");
    }

    #[test]
    fn v2_skipped_documents_excluded() {
        use crate::report::{BatchReport, BatchSummary, DocumentOutcome, DocumentResult};
        use std::path::PathBuf;

        let docs = vec![DocumentResult {
            input_index: 0,
            source_label: "x.pdf".into(),
            output_dir: PathBuf::from("./out"),
            outcome: DocumentOutcome::Skipped,
        }];
        let report = BatchReport {
            summary: BatchSummary::from_documents(&docs),
            documents: docs,
        };
        assert!(build_v2(&report).is_none());
    }

    #[test]
    fn v2_empty_or_all_success_yields_none() {
        use crate::report::{BatchReport, BatchSummary};

        let empty = BatchReport {
            documents: vec![],
            summary: BatchSummary::default(),
        };
        assert!(build_v2(&empty).is_none());
    }
}
