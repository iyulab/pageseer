//! `HWP`/`HWPX` → in-memory `PDF` 어댑터 — spec §5.3.
//!
//! 경로: `HWP` bytes → `rhwp::wasm_api::HwpDocument` → 페이지별 `SVG` →
//! `rhwp::renderer::pdf::svgs_to_pdf` → `PDF` bytes.

use std::path::Path;

use crate::error::PageseerError;

/// `HWP`/`HWPX` 파일 1개 → `PDF` bytes (in-memory).
///
/// 한글 폰트 부재 시 `rhwp` 내부 fallback 발생 (시각 품질 저하 가능).
/// 손상된 `HWP`/암호화 파일은 `PageseerError::Rhwp`로 분류된다.
pub fn convert_to_pdf_bytes(path: &Path) -> Result<Vec<u8>, PageseerError> {
    let bytes = std::fs::read(path).map_err(|e| PageseerError::Rhwp(format!("read input: {e}")))?;
    convert_bytes_to_pdf(&bytes)
}

/// 메모리 `HWP` bytes → `PDF` bytes.
pub(crate) fn convert_bytes_to_pdf(bytes: &[u8]) -> Result<Vec<u8>, PageseerError> {
    let doc = rhwp::wasm_api::HwpDocument::from_bytes(bytes)
        .map_err(|e| PageseerError::Rhwp(format!("parse hwp: {e}")))?;
    let page_count = doc.page_count();
    if page_count == 0 {
        return Err(PageseerError::Rhwp("hwp has zero pages".to_owned()));
    }
    let mut svgs: Vec<String> = Vec::with_capacity(page_count as usize);
    for i in 0..page_count {
        let svg = doc
            .render_page_svg(i)
            .map_err(|e| PageseerError::Rhwp(format!("render svg page {i}: {e:?}")))?;
        svgs.push(svg);
    }
    rhwp::renderer::pdf::svgs_to_pdf(&svgs)
        .map_err(|e| PageseerError::Rhwp(format!("svgs to pdf: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bytes_returns_rhwp_parse_error() {
        let result = convert_bytes_to_pdf(&[]);
        match result {
            Err(PageseerError::Rhwp(msg)) => {
                assert!(msg.contains("parse hwp"), "message: {msg}");
            }
            other => panic!("expected Rhwp parse error, got {other:?}"),
        }
    }

    #[test]
    fn nonexistent_path_returns_rhwp_read_error() {
        let result = convert_to_pdf_bytes(Path::new("nonexistent-hwp-fixture.hwp"));
        match result {
            Err(PageseerError::Rhwp(msg)) => {
                assert!(msg.contains("read input"), "message: {msg}");
            }
            other => panic!("expected Rhwp read error, got {other:?}"),
        }
    }
}
