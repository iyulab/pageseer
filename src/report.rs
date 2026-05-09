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

/// 배치 처리 전체 집계 요약.
#[derive(Debug, Clone, Copy, Default)]
pub struct BatchSummary {
    /// 처리 대상 문서 수 (dedup 후).
    pub documents_total: u32,
    /// 모든 페이지가 성공한 문서 수.
    pub documents_succeeded: u32,
    /// 일부 페이지만 성공한 문서 수.
    pub documents_partial: u32,
    /// 단 한 페이지도 성공하지 못한 문서 수.
    pub documents_failed: u32,
    /// `--strict` cancel로 건너뛴 문서 수.
    pub documents_skipped: u32,
    /// 전체 성공 페이지 수.
    pub pages_succeeded: u32,
    /// 전체 실패 페이지 수.
    pub pages_failed: u32,
}

impl BatchSummary {
    /// `DocumentResult` 슬라이스로부터 카운트 요약을 산출한다.
    #[must_use]
    pub fn from_documents(docs: &[DocumentResult]) -> Self {
        let mut s = Self {
            documents_total: u32::try_from(docs.len()).unwrap_or(u32::MAX),
            ..Self::default()
        };
        for d in docs {
            match &d.outcome {
                DocumentOutcome::Processed(report) => {
                    if report.failed.is_empty() {
                        s.documents_succeeded = s.documents_succeeded.saturating_add(1);
                    } else {
                        s.documents_partial = s.documents_partial.saturating_add(1);
                    }
                    s.pages_succeeded = s
                        .pages_succeeded
                        .saturating_add(u32::try_from(report.succeeded.len()).unwrap_or(u32::MAX));
                    s.pages_failed = s
                        .pages_failed
                        .saturating_add(u32::try_from(report.failed.len()).unwrap_or(u32::MAX));
                }
                DocumentOutcome::Failed(_) => {
                    s.documents_failed = s.documents_failed.saturating_add(1);
                }
                DocumentOutcome::Skipped => {
                    s.documents_skipped = s.documents_skipped.saturating_add(1);
                }
            }
        }
        s
    }
}

/// 한 입력 문서의 처리 결과.
#[derive(Debug, Clone)]
pub struct DocumentResult {
    /// 입력 `Vec<SourceInput>`에서의 위치 (0-based).
    pub input_index: u32,
    /// 표시용 라벨. Path 입력은 경로 문자열, Bytes 입력은 filename hint.
    pub source_label: String,
    /// 이 문서의 산출물이 모인 디렉터리 (dedup이 적용된 최종 경로).
    pub output_dir: PathBuf,
    /// 처리 결과.
    pub outcome: DocumentOutcome,
}

/// 한 문서가 어떻게 끝났는지.
#[derive(Debug, Clone)]
pub enum DocumentOutcome {
    /// 페이지 단계까지 도달 — 일부/전부 페이지 성공/실패 가능.
    Processed(ExtractReport),
    /// 문서 단위 치명적 실패 (`SourceRead` / `Convert` 단계).
    Failed(PageFailure),
    /// strict 모드에서 다른 문서가 먼저 실패해 시도되지 않음.
    Skipped,
}

/// 배치 처리 결과 — `extract`의 반환 타입.
#[derive(Debug, Clone)]
pub struct BatchReport {
    /// 입력 순서대로의 문서별 결과.
    pub documents: Vec<DocumentResult>,
    /// 문서/페이지 카운트 요약.
    pub summary: BatchSummary,
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

    #[test]
    fn batch_summary_default_is_all_zero() {
        let s = BatchSummary::default();
        assert_eq!(s.documents_total, 0);
        assert_eq!(s.documents_succeeded, 0);
        assert_eq!(s.documents_partial, 0);
        assert_eq!(s.documents_failed, 0);
        assert_eq!(s.documents_skipped, 0);
        assert_eq!(s.pages_succeeded, 0);
        assert_eq!(s.pages_failed, 0);
    }

    #[test]
    fn batch_summary_fields_settable() {
        let s = BatchSummary {
            documents_total: 3,
            documents_succeeded: 1,
            documents_partial: 1,
            documents_failed: 1,
            documents_skipped: 0,
            pages_succeeded: 29,
            pages_failed: 1,
        };
        assert_eq!(s.documents_total, 3);
        assert_eq!(s.pages_succeeded, 29);
    }

    #[test]
    fn document_outcome_processed_holds_extract_report() {
        let mut inner = ExtractReport::new();
        inner.failed.push(PageFailure {
            source_path: None,
            page_index: Some(0),
            stage: FailureStage::Rasterize,
            message: "x".into(),
        });
        let outcome = DocumentOutcome::Processed(inner);
        match outcome {
            DocumentOutcome::Processed(r) => assert_eq!(r.failed_count(), 1),
            _ => panic!("expected Processed"),
        }
    }

    #[test]
    fn document_outcome_failed_holds_page_failure() {
        let f = PageFailure {
            source_path: None,
            page_index: None,
            stage: FailureStage::Convert,
            message: "boom".into(),
        };
        let outcome = DocumentOutcome::Failed(f);
        assert!(matches!(outcome, DocumentOutcome::Failed(_)));
    }

    #[test]
    fn document_outcome_skipped_is_constructable() {
        let outcome = DocumentOutcome::Skipped;
        assert!(matches!(outcome, DocumentOutcome::Skipped));
    }

    #[test]
    fn batch_report_documents_field_writable() {
        let report = BatchReport {
            documents: vec![],
            summary: BatchSummary::default(),
        };
        assert_eq!(report.documents.len(), 0);
    }

    #[test]
    fn batch_summary_from_documents_counts_processed_only() {
        use std::path::PathBuf;
        let mut full = ExtractReport::new();
        full.succeeded.push(PageArtifact {
            source_path: None,
            page_index: 0,
            output_path: PathBuf::from("a.png"),
            width_px: 10,
            height_px: 10,
        });
        full.succeeded.push(PageArtifact {
            source_path: None,
            page_index: 1,
            output_path: PathBuf::from("b.png"),
            width_px: 10,
            height_px: 10,
        });
        let mut partial = ExtractReport::new();
        partial.succeeded.push(PageArtifact {
            source_path: None,
            page_index: 0,
            output_path: PathBuf::from("c.png"),
            width_px: 10,
            height_px: 10,
        });
        partial.failed.push(PageFailure {
            source_path: None,
            page_index: Some(1),
            stage: FailureStage::Rasterize,
            message: "x".into(),
        });
        let docs = vec![
            DocumentResult {
                input_index: 0,
                source_label: "a".into(),
                output_dir: PathBuf::from("a"),
                outcome: DocumentOutcome::Processed(full),
            },
            DocumentResult {
                input_index: 1,
                source_label: "b".into(),
                output_dir: PathBuf::from("b"),
                outcome: DocumentOutcome::Processed(partial),
            },
            DocumentResult {
                input_index: 2,
                source_label: "c".into(),
                output_dir: PathBuf::from("c"),
                outcome: DocumentOutcome::Failed(PageFailure {
                    source_path: None,
                    page_index: None,
                    stage: FailureStage::Convert,
                    message: "x".into(),
                }),
            },
            DocumentResult {
                input_index: 3,
                source_label: "d".into(),
                output_dir: PathBuf::from("d"),
                outcome: DocumentOutcome::Skipped,
            },
        ];
        let s = BatchSummary::from_documents(&docs);
        assert_eq!(s.documents_total, 4);
        assert_eq!(s.documents_succeeded, 1);
        assert_eq!(s.documents_partial, 1);
        assert_eq!(s.documents_failed, 1);
        assert_eq!(s.documents_skipped, 1);
        assert_eq!(s.pages_succeeded, 3);
        assert_eq!(s.pages_failed, 1);
    }
}
