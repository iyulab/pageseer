//! `errors.json` 직렬화 (v2 스키마) — spec §4.

use std::path::Path;

use serde::Serialize;

use crate::report::{BatchReport, BatchSummary, DocumentOutcome, DocumentResult, PageFailure};

/// `errors.json` v2 루트 스키마.
#[derive(Debug, Serialize)]
pub struct ErrorsReport {
    /// 스키마 버전 (현재 2).
    pub version: u32,
    /// 배치 카운트 요약.
    pub summary: BatchSummary,
    /// 실패 항목.
    pub errors: Vec<ErrorEntry>,
}

/// 단일 실패 항목.
#[derive(Debug, Serialize)]
pub struct ErrorEntry {
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
pub fn build(report: &BatchReport) -> Option<ErrorsReport> {
    let mut errors: Vec<ErrorEntry> = Vec::new();
    for doc in &report.documents {
        match &doc.outcome {
            DocumentOutcome::Processed(inner) => {
                for f in &inner.failed {
                    errors.push(entry_for(doc, f));
                }
            }
            DocumentOutcome::Failed(f) => {
                errors.push(entry_for(doc, f));
            }
            DocumentOutcome::Skipped => {}
        }
    }
    if errors.is_empty() {
        return None;
    }
    Some(ErrorsReport {
        version: 2,
        summary: report.summary,
        errors,
    })
}

fn entry_for(doc: &DocumentResult, f: &PageFailure) -> ErrorEntry {
    ErrorEntry {
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

/// `<dir>/errors.json` 작성. 실패 0건이면 false 반환.
pub fn write_to_dir(report: &BatchReport, dir: &Path) -> std::io::Result<bool> {
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
    use crate::report::{
        BatchReport, BatchSummary, DocumentOutcome, DocumentResult, ExtractReport, FailureStage,
        PageFailure,
    };
    use std::path::PathBuf;

    fn doc_processed(idx: u32, label: &str, inner: ExtractReport) -> DocumentResult {
        DocumentResult {
            input_index: idx,
            source_label: label.into(),
            output_dir: PathBuf::from("./out"),
            outcome: DocumentOutcome::Processed(inner),
        }
    }

    #[test]
    fn empty_report_yields_none() {
        let r = BatchReport {
            documents: vec![],
            summary: BatchSummary::default(),
        };
        assert!(build(&r).is_none());
    }

    #[test]
    fn page_index_serialized_as_one_based() {
        let mut inner = ExtractReport::new();
        inner.failed.push(PageFailure {
            source_path: Some(PathBuf::from("a.pdf")),
            page_index: Some(0),
            stage: FailureStage::Rasterize,
            message: "boom".into(),
        });
        let docs = vec![doc_processed(0, "a.pdf", inner)];
        let report = BatchReport {
            summary: BatchSummary::from_documents(&docs),
            documents: docs,
        };
        let payload = build(&report).unwrap();
        assert_eq!(payload.version, 2);
        assert_eq!(payload.errors[0].document_index, 0);
        assert_eq!(payload.errors[0].page, Some(1));
        assert_eq!(payload.errors[0].stage, "rasterize");
        assert_eq!(payload.errors[0].source, "a.pdf");
    }

    #[test]
    fn document_level_failed_outcome_serializes_with_null_page() {
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
        let payload = build(&report).unwrap();
        assert_eq!(payload.errors[0].document_index, 1);
        assert_eq!(payload.errors[0].page, None);
        assert_eq!(payload.errors[0].stage, "convert");
    }

    #[test]
    fn skipped_documents_excluded() {
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
        assert!(build(&report).is_none());
    }
}
