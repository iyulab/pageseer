//! pageseer — document-to-page-image rasterizer.
//!
//! See the design spec at `claudedocs/specs/` for architecture.
//!
//! # Status
//!
//! S1 (PDF-only end-to-end). `SourceInput::Bytes`와 `Office`/`HWP` 입력은 후속 슬라이스.

#![warn(missing_docs)]

use std::path::{Path, PathBuf};

pub mod error;
pub mod errors_json;
pub mod format;
pub mod options;
pub mod output;
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
/// - `PageseerError::Pdfium` — `PDFium` 라이브러리 로드/렌더 실패.
/// - `PageseerError::UnsupportedFormat` — 알 수 없거나 S1 미지원 포맷.
/// - `PageseerError::Config` — `SourceInput::Bytes`(S1 미지원) 등 호출 측 오류.
/// - `PageseerError::Io` — 디렉터리 생성/파일 쓰기 실패.
pub fn extract(input: SourceInput, options: Options) -> Result<ExtractReport, PageseerError> {
    let options = options.normalized();
    let path = match input {
        SourceInput::Path(p) => p,
        SourceInput::Bytes { .. } => {
            return Err(PageseerError::Config(
                "SourceInput::Bytes not supported in S1 (PDF path input only)".to_owned(),
            ));
        }
    };

    match format::detect_from_path(&path) {
        format::DetectedFormat::Pdf => extract_pdf(&path, &options),
        format::DetectedFormat::Other => Err(PageseerError::UnsupportedFormat {
            extension: path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_owned(),
            path: Some(path),
        }),
    }
}

fn extract_pdf(path: &Path, options: &Options) -> Result<ExtractReport, PageseerError> {
    use image::ImageFormat as ImgFmt;

    let backend = raster::pdfium::PdfiumBackend::new()?;
    let page_results = backend.rasterize_path_pages(path, options.dpi)?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    let page_count = page_results.len();

    let target_dir = if options.flat {
        options.output_dir.clone()
    } else {
        options.output_dir.join(&stem)
    };
    std::fs::create_dir_all(&target_dir)?;

    let img_format = match options.format {
        ImageFormat::Png => ImgFmt::Png,
        ImageFormat::Jpeg { .. } => ImgFmt::Jpeg,
    };

    let mut report = ExtractReport::new();
    for (idx, page_result) in page_results.into_iter().enumerate() {
        let idx_u32 = u32::try_from(idx)
            .map_err(|_| PageseerError::Pdfium(format!("page index {idx} exceeds u32::MAX")))?;
        let img = match page_result {
            Ok(img) => img,
            Err(e) => {
                if options.strict {
                    return Err(e);
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
            &options.output_dir,
            &stem,
            idx_u32,
            page_count,
            options.format,
            options.flat,
        );
        let to_save: image::DynamicImage = match options.format {
            ImageFormat::Jpeg { .. } => img.into_rgb8().into(),
            ImageFormat::Png => img,
        };
        let (w, h) = (to_save.width(), to_save.height());
        match to_save.save_with_format(&out, img_format) {
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
                    return Err(PageseerError::Io(std::io::Error::other(e.to_string())));
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

    if !report.failed.is_empty() {
        errors_json::write_to_dir(&report, &target_dir).map_err(PageseerError::Io)?;
        return Err(PageseerError::Partial(report));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_path_with_unknown_extension_returns_unsupported() {
        let result = extract(
            SourceInput::Path(PathBuf::from("nonexistent.xyz")),
            Options::default(),
        );
        match result {
            Err(PageseerError::UnsupportedFormat { extension, .. }) => {
                assert_eq!(extension, "xyz");
            }
            other => panic!("expected UnsupportedFormat, got {other:?}"),
        }
    }

    #[test]
    fn extract_bytes_returns_config_error_in_s1() {
        let result = extract(
            SourceInput::Bytes {
                data: vec![1, 2, 3],
                filename: "x.pdf".to_owned(),
            },
            Options::default(),
        );
        assert!(matches!(result, Err(PageseerError::Config(_))));
    }
}
