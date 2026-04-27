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
pub mod gotenberg;
pub mod hwp;
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
                "SourceInput::Bytes not supported in v0.1 (path input only)".to_owned(),
            ));
        }
    };

    match format::detect_from_path(&path) {
        format::DetectedFormat::Pdf => extract_pdf(&path, &options),
        format::DetectedFormat::Office => extract_office(&path, &options),
        format::DetectedFormat::Hwp => extract_hwp(&path, &options),
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

fn extract_hwp(path: &Path, options: &Options) -> Result<ExtractReport, PageseerError> {
    let pdf_bytes = hwp::convert_to_pdf_bytes(path)?;

    let original_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    let tmp_pdf = std::env::temp_dir().join(format!(
        "pageseer-hwp-{}-{original_stem}.pdf",
        std::process::id()
    ));
    std::fs::write(&tmp_pdf, &pdf_bytes)?;

    let mut result = extract_pdf_with_stem(&tmp_pdf, &original_stem, options);
    let _ = std::fs::remove_file(&tmp_pdf);

    rewrite_source_paths(&mut result, path);
    result
}

fn extract_office(path: &Path, options: &Options) -> Result<ExtractReport, PageseerError> {
    let base = gotenberg::resolve_base_url(options.gotenberg_url.as_deref());
    let client = gotenberg::GotenbergClient::new(base, options.gotenberg_timeout)?;
    let pdf_bytes = client.convert_office(path)?;

    let original_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    let tmp_pdf = std::env::temp_dir().join(format!(
        "pageseer-gotenberg-{}-{original_stem}.pdf",
        std::process::id()
    ));
    std::fs::write(&tmp_pdf, &pdf_bytes)?;

    let mut result = extract_pdf_with_stem(&tmp_pdf, &original_stem, options);
    let _ = std::fs::remove_file(&tmp_pdf);

    // tmp_pdf 경로를 원본 path로 rewrite (사용자에게 노출되는 path는 원본).
    rewrite_source_paths(&mut result, path);
    result
}

fn rewrite_source_paths(result: &mut Result<ExtractReport, PageseerError>, original: &Path) {
    let (Ok(report) | Err(PageseerError::Partial(report))) = result else {
        return;
    };
    for art in &mut report.succeeded {
        art.source_path = Some(original.to_path_buf());
    }
    for f in &mut report.failed {
        f.source_path = Some(original.to_path_buf());
    }
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

fn extract_pdf(path: &Path, options: &Options) -> Result<ExtractReport, PageseerError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_owned();
    extract_pdf_with_stem(path, &stem, options)
}

fn extract_pdf_with_stem(
    path: &Path,
    stem: &str,
    options: &Options,
) -> Result<ExtractReport, PageseerError> {
    let backend = raster::pdfium::PdfiumBackend::new()?;
    let page_results = backend.rasterize_path_pages(path, options.dpi)?;
    let page_count = page_results.len();

    let target_dir = if options.flat {
        options.output_dir.clone()
    } else {
        options.output_dir.join(stem)
    };
    std::fs::create_dir_all(&target_dir)?;

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
        let save_result = save_image(&to_save, &out, options.format);
        match save_result {
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

    #[test]
    fn save_image_jpeg_writes_valid_jfif_magic() {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 2, image::Rgb([200, 100, 50])));
        let tmp = std::env::temp_dir().join(format!(
            "pageseer-jpeg-test-{}-{}.jpg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
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
                .map(|d| d.as_nanos())
                .unwrap_or(0)
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

    #[test]
    fn extract_hwp_path_nonexistent_returns_rhwp_read_error() {
        let result = extract(
            SourceInput::Path(PathBuf::from("nonexistent.hwp")),
            Options::default(),
        );
        match result {
            Err(PageseerError::Rhwp(msg)) => {
                assert!(msg.contains("read input"), "unexpected message: {msg}");
            }
            other => panic!("expected Rhwp read-error, got {other:?}"),
        }
    }
}
