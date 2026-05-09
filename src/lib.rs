//! pageseer — document-to-page-image rasterizer.
//!
//! See the design spec at `claudedocs/specs/` for architecture.

#![warn(missing_docs)]

use std::path::{Path, PathBuf};

pub mod error;
pub mod errors_json;
pub mod format;
pub mod gotenberg;
pub mod hwp;
pub mod options;
pub mod output;
pub mod raster;
pub mod report;

mod batch;

pub use batch::{NullProgressSink, ProgressSink, StderrProgressSink};
pub use error::PageseerError;
pub use options::{ImageFormat, Options};
pub use report::{
    BatchReport, BatchSummary, DocumentOutcome, DocumentResult, ExtractReport, FailureStage,
    PageArtifact, PageFailure,
};

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

/// 다중 입력 배치를 페이지 이미지로 추출한다.
///
/// # Errors
///
/// init 단계 오류만 `Err`로 반환:
/// - `PageseerError::Config` — 빈 입력, flat 모드 stem 충돌.
/// - `PageseerError::Io` — `output_dir` 생성 실패.
///
/// 입력별 처리 결과(미지원 포맷, 읽기 실패 등)는 모두
/// `Ok(BatchReport)` 안의 `DocumentResult.outcome`으로 표현된다.
pub fn extract(inputs: &[SourceInput], options: Options) -> Result<BatchReport, PageseerError> {
    extract_with_progress(inputs, options, &NullProgressSink)
}

/// `extract`와 동일하지만 진행 상황 콜백을 받는다 (`CLI`/`FFI`에서 사용).
pub fn extract_with_progress(
    inputs: &[SourceInput],
    options: Options,
    progress: &(dyn ProgressSink + Sync),
) -> Result<BatchReport, PageseerError> {
    if inputs.is_empty() {
        return Err(PageseerError::Config("empty input list".to_owned()));
    }
    let options = options.normalized();
    let mapping = batch::dedup_output_dirs(inputs, &options.output_dir, options.flat)?;
    std::fs::create_dir_all(&options.output_dir)?;

    let total = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
    let mut documents: Vec<DocumentResult> = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let idx = u32::try_from(i).unwrap_or(u32::MAX);
        let label = batch::source_label(input);
        progress.record_start(idx, total, &label);
        let outcome = process_one_document(input, &mapping[i], &options);
        report_progress(progress, idx, total, &label, &outcome);
        documents.push(DocumentResult {
            input_index: idx,
            source_label: label,
            output_dir: mapping[i].clone(),
            outcome,
        });
    }

    let summary = BatchSummary::from_documents(&documents);
    let report = BatchReport { documents, summary };
    progress.record_summary(&report.summary);

    if report.summary.pages_failed > 0 || report.summary.documents_failed > 0 {
        errors_json::write_to_dir(&report, &options.output_dir).map_err(PageseerError::Io)?;
    }
    Ok(report)
}

fn report_progress(
    progress: &(dyn ProgressSink + Sync),
    idx: u32,
    total: u32,
    label: &str,
    outcome: &DocumentOutcome,
) {
    match outcome {
        DocumentOutcome::Processed(r) => progress.record_done(
            idx,
            total,
            label,
            u32::try_from(r.succeeded.len()).unwrap_or(u32::MAX),
            u32::try_from(r.failed.len()).unwrap_or(u32::MAX),
        ),
        DocumentOutcome::Failed(f) => progress.record_failed(idx, total, label, &f.message),
        DocumentOutcome::Skipped => progress.record_skipped(idx, total, label),
    }
}

fn process_one_document(
    input: &SourceInput,
    target_dir: &Path,
    options: &Options,
) -> DocumentOutcome {
    let path = match input {
        SourceInput::Path(p) => p.clone(),
        SourceInput::Bytes { filename, .. } => {
            return DocumentOutcome::Failed(PageFailure {
                source_path: None,
                page_index: None,
                stage: FailureStage::SourceRead,
                message: format!(
                    "SourceInput::Bytes not supported in v0.2-A (filename={filename})"
                ),
            });
        }
    };

    match format::detect_from_path(&path) {
        format::DetectedFormat::Pdf => process_pdf(&path, target_dir, options),
        format::DetectedFormat::Office => process_office(&path, target_dir, options),
        format::DetectedFormat::Hwp => process_hwp(&path, target_dir, options),
        format::DetectedFormat::Other => DocumentOutcome::Failed(PageFailure {
            source_path: Some(path.clone()),
            page_index: None,
            stage: FailureStage::SourceRead,
            message: format!(
                "unsupported format: .{}",
                path.extension().and_then(|s| s.to_str()).unwrap_or("")
            ),
        }),
    }
}

fn process_pdf(path: &Path, target_dir: &Path, options: &Options) -> DocumentOutcome {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    rasterize_pdf_into_outcome(path, &stem, target_dir, options)
}

fn process_office(path: &Path, target_dir: &Path, options: &Options) -> DocumentOutcome {
    let base = gotenberg::resolve_base_url(options.gotenberg_url.as_deref());
    let client = match gotenberg::GotenbergClient::new(base, options.gotenberg_timeout) {
        Ok(c) => c,
        Err(e) => return failed_outcome(path, FailureStage::Convert, e.to_string()),
    };
    let pdf_bytes = match client.convert_office(path) {
        Ok(b) => b,
        Err(e) => return failed_outcome(path, FailureStage::Convert, e.to_string()),
    };
    let original_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    let tmp_pdf = std::env::temp_dir().join(format!(
        "pageseer-gotenberg-{}-{original_stem}.pdf",
        std::process::id()
    ));
    if let Err(e) = std::fs::write(&tmp_pdf, &pdf_bytes) {
        return failed_outcome(path, FailureStage::Convert, e.to_string());
    }
    let outcome = rasterize_pdf_into_outcome(&tmp_pdf, &original_stem, target_dir, options);
    let _ = std::fs::remove_file(&tmp_pdf);
    rewrite_source_paths_in_outcome(outcome, path)
}

fn process_hwp(path: &Path, target_dir: &Path, options: &Options) -> DocumentOutcome {
    let pdf_bytes = match hwp::convert_to_pdf_bytes(path) {
        Ok(b) => b,
        Err(e) => return failed_outcome(path, FailureStage::SourceRead, e.to_string()),
    };
    let original_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    let tmp_pdf = std::env::temp_dir().join(format!(
        "pageseer-hwp-{}-{original_stem}.pdf",
        std::process::id()
    ));
    if let Err(e) = std::fs::write(&tmp_pdf, &pdf_bytes) {
        return failed_outcome(path, FailureStage::Convert, e.to_string());
    }
    let outcome = rasterize_pdf_into_outcome(&tmp_pdf, &original_stem, target_dir, options);
    let _ = std::fs::remove_file(&tmp_pdf);
    rewrite_source_paths_in_outcome(outcome, path)
}

fn failed_outcome(path: &Path, stage: FailureStage, message: String) -> DocumentOutcome {
    DocumentOutcome::Failed(PageFailure {
        source_path: Some(path.to_path_buf()),
        page_index: None,
        stage,
        message,
    })
}

fn rewrite_source_paths_in_outcome(outcome: DocumentOutcome, original: &Path) -> DocumentOutcome {
    match outcome {
        DocumentOutcome::Processed(mut r) => {
            for art in &mut r.succeeded {
                art.source_path = Some(original.to_path_buf());
            }
            for f in &mut r.failed {
                f.source_path = Some(original.to_path_buf());
            }
            DocumentOutcome::Processed(r)
        }
        DocumentOutcome::Failed(mut f) => {
            f.source_path = Some(original.to_path_buf());
            DocumentOutcome::Failed(f)
        }
        DocumentOutcome::Skipped => DocumentOutcome::Skipped,
    }
}

fn rasterize_pdf_into_outcome(
    path: &Path,
    stem: &str,
    target_dir: &Path,
    options: &Options,
) -> DocumentOutcome {
    let backend = match raster::pdfium::PdfiumBackend::new() {
        Ok(b) => b,
        Err(e) => return failed_outcome(path, FailureStage::Rasterize, e.to_string()),
    };
    let page_results = match backend.rasterize_path_pages(path, options.dpi) {
        Ok(r) => r,
        Err(e) => return failed_outcome(path, FailureStage::Rasterize, e.to_string()),
    };
    let page_count = page_results.len();

    if let Err(e) = std::fs::create_dir_all(target_dir) {
        return failed_outcome(path, FailureStage::Write, e.to_string());
    }

    let mut report = ExtractReport::new();
    for (idx, page_result) in page_results.into_iter().enumerate() {
        let Ok(idx_u32) = u32::try_from(idx) else {
            return failed_outcome(
                path,
                FailureStage::Rasterize,
                format!("page index {idx} exceeds u32::MAX"),
            );
        };
        let img = match page_result {
            Ok(img) => img,
            Err(e) => {
                if options.strict {
                    return DocumentOutcome::Processed(report);
                }
                report.failed.push(PageFailure {
                    source_path: Some(path.to_path_buf()),
                    page_index: Some(idx_u32),
                    stage: FailureStage::Rasterize,
                    message: e.to_string(),
                });
                continue;
            }
        };
        let out = output::page_output_path(
            target_dir,
            stem,
            idx_u32,
            page_count,
            options.format,
            options.flat,
        );
        let scaled = raster::apply_max_edge(img, options.max_edge);
        let to_save: image::DynamicImage = match options.format {
            ImageFormat::Jpeg { .. } => scaled.into_rgb8().into(),
            ImageFormat::Png => scaled,
        };
        let (w, h) = (to_save.width(), to_save.height());
        match save_image(&to_save, &out, options.format) {
            Ok(()) => {
                report.succeeded.push(PageArtifact {
                    source_path: Some(path.to_path_buf()),
                    page_index: idx_u32,
                    output_path: out,
                    width_px: w,
                    height_px: h,
                });
            }
            Err(e) => {
                if options.strict {
                    return DocumentOutcome::Processed(report);
                }
                report.failed.push(PageFailure {
                    source_path: Some(path.to_path_buf()),
                    page_index: Some(idx_u32),
                    stage: FailureStage::Write,
                    message: e.to_string(),
                });
            }
        }
    }
    DocumentOutcome::Processed(report)
}

fn save_image(
    img: &image::DynamicImage,
    path: &Path,
    format: ImageFormat,
) -> image::ImageResult<()> {
    match format {
        ImageFormat::Png => img.save_with_format(path, image::ImageFormat::Png),
        ImageFormat::Jpeg { quality } => {
            let file = std::fs::File::create(path)?;
            let mut writer = std::io::BufWriter::new(file);
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
            encoder.encode_image(img)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_vec_returns_config_error() {
        let result = extract(&[], Options::default());
        assert!(matches!(result, Err(PageseerError::Config(_))));
    }

    #[test]
    fn unknown_extension_yields_failed_outcome() {
        let report = extract(
            &[SourceInput::Path(PathBuf::from("nonexistent.xyz"))],
            Options::default(),
        )
        .expect("extract should not return Err for unsupported format");
        assert_eq!(report.documents.len(), 1);
        match &report.documents[0].outcome {
            DocumentOutcome::Failed(f) => {
                assert_eq!(f.stage, FailureStage::SourceRead);
                assert!(f.message.contains("unsupported"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(report.summary.documents_failed, 1);
    }

    #[test]
    fn bytes_input_yields_failed_outcome_in_v0_2_a() {
        let report = extract(
            &[SourceInput::Bytes {
                data: vec![1, 2, 3],
                filename: "x.pdf".into(),
            }],
            Options::default(),
        )
        .expect("extract should not return Err for unsupported input form");
        assert_eq!(report.summary.documents_failed, 1);
        assert!(matches!(
            report.documents[0].outcome,
            DocumentOutcome::Failed(_)
        ));
    }

    #[test]
    fn nonexistent_hwp_yields_failed_outcome() {
        let report = extract(
            &[SourceInput::Path(PathBuf::from("nonexistent.hwp"))],
            Options::default(),
        )
        .expect("extract should not return Err for HWP read error");
        assert_eq!(report.summary.documents_failed, 1);
        match &report.documents[0].outcome {
            DocumentOutcome::Failed(f) => {
                assert_eq!(f.stage, FailureStage::SourceRead);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn save_image_jpeg_writes_valid_jfif_magic() {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 2, image::Rgb([200, 100, 50])));
        let tmp = std::env::temp_dir().join(format!(
            "pageseer-jpeg-test-{}-{}.jpg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        save_image(&img, &tmp, ImageFormat::Jpeg { quality: 85 }).expect("save jpeg");
        let bytes = std::fs::read(&tmp).expect("read jpeg");
        let _ = std::fs::remove_file(&tmp);
        assert!(bytes.len() >= 3, "jpeg too small");
        assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF], "missing JPEG SOI marker");
    }

    #[test]
    fn save_image_jpeg_quality_affects_file_size() {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::from_fn(64, 64, |x, y| {
            let v = u8::try_from((x * 3 + y * 5) % 256).unwrap_or(0);
            image::Rgb([v, v.wrapping_mul(3), v.wrapping_mul(7)])
        }));
        let dir = std::env::temp_dir().join(format!(
            "pageseer-jpeg-q-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p_low = dir.join("q10.jpg");
        let p_high = dir.join("q95.jpg");
        save_image(&img, &p_low, ImageFormat::Jpeg { quality: 10 }).unwrap();
        save_image(&img, &p_high, ImageFormat::Jpeg { quality: 95 }).unwrap();
        let sz_low = std::fs::metadata(&p_low).unwrap().len();
        let sz_high = std::fs::metadata(&p_high).unwrap().len();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            sz_high > sz_low,
            "expected q95 > q10 in size; got high={sz_high} low={sz_low}"
        );
    }
}
