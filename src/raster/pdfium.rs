//! `PDFium` 라스터 백엔드 (pdfium-render 0.9 wrapping).

use std::path::Path;

use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::error::PageseerError;

/// 단일 `PDFium` 인스턴스. 전체 프로세스에서 1개만 생성 (`FFI` 내부 mutex로 직렬화됨).
pub struct PdfiumBackend {
    pdfium: Pdfium,
}

impl PdfiumBackend {
    /// 1순위 `./pdfium/` 디렉터리, 2순위 시스템 경로에서 라이브러리를 찾는다.
    pub fn new() -> Result<Self, PageseerError> {
        let bindings =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./pdfium/"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| PageseerError::Pdfium(format!("library load failed: {e}")))?;
        Ok(Self {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// `PDF` 파일을 페이지별 `DynamicImage`로 라스터화.
    pub fn rasterize_path(
        &self,
        path: &Path,
        dpi: u32,
    ) -> Result<Vec<DynamicImage>, PageseerError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PageseerError::Pdfium(format!("load_pdf_from_file: {e}")))?;
        Self::rasterize_document(&document, dpi)
    }

    fn rasterize_document(
        document: &PdfDocument,
        dpi: u32,
    ) -> Result<Vec<DynamicImage>, PageseerError> {
        let mut out = Vec::new();
        for page in document.pages().iter() {
            let target_width = super::pixels_from_points(page.width().value, dpi);
            let cfg = PdfRenderConfig::new().set_target_width(
                i32::try_from(target_width).map_err(|_| {
                    PageseerError::Pdfium(format!("target width {target_width}px exceeds i32::MAX"))
                })?,
            );
            let img = page
                .render_with_config(&cfg)
                .map_err(|e| PageseerError::Pdfium(format!("render: {e}")))?
                .as_image()
                .map_err(|e| PageseerError::Pdfium(format!("as_image: {e}")))?;
            out.push(img);
        }
        Ok(out)
    }
}
