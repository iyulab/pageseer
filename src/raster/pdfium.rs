//! `PDFium` 라스터 백엔드 (pdfium-render 0.9 wrapping).

use std::path::Path;
use std::sync::OnceLock;

use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::error::PageseerError;

static PDFIUM: OnceLock<Result<PdfiumBackend, String>> = OnceLock::new();

/// process-global 싱글톤 `PdfiumBackend`를 반환한다.
///
/// 최초 호출 시 라이브러리를 로드하고 `FPDF_InitLibrary()`를 호출한다.
/// 이후 호출은 동일 결과를 즉시 반환하므로 다중 스레드에서 동시 호출해도 안전하다.
pub fn global() -> Result<&'static PdfiumBackend, PageseerError> {
    PDFIUM
        .get_or_init(|| PdfiumBackend::init().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| PageseerError::Pdfium(e.clone()))
}

/// `FPDF_InitLibrary()`가 process 당 정확히 1회 호출되도록 보장하는 백엔드.
pub struct PdfiumBackend {
    pdfium: Pdfium,
}

impl PdfiumBackend {
    fn init() -> Result<Self, PageseerError> {
        let bindings =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./pdfium/"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| PageseerError::Pdfium(format!("library load failed: {e}")))?;
        Ok(Self {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// `PDF` 파일을 페이지별 `DynamicImage`로 라스터화 (실패시 첫 에러 반환).
    pub fn rasterize_path(
        &self,
        path: &Path,
        dpi: u32,
    ) -> Result<Vec<DynamicImage>, PageseerError> {
        self.rasterize_path_pages(path, dpi)?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
    }

    /// 페이지 단위 결과 vector — strict/continue 분기 지원.
    /// 문서 로드 실패는 외부 `Err`, 페이지별 결과는 내부 `Result`.
    pub fn rasterize_path_pages(
        &self,
        path: &Path,
        dpi: u32,
    ) -> Result<Vec<Result<DynamicImage, PageseerError>>, PageseerError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| PageseerError::Pdfium(format!("load_pdf_from_file: {e}")))?;
        let mut out = Vec::new();
        for page in document.pages().iter() {
            out.push(Self::render_one(&page, dpi));
        }
        Ok(out)
    }

    fn render_one(page: &PdfPage, dpi: u32) -> Result<DynamicImage, PageseerError> {
        let target_width = super::pixels_from_points(page.width().value, dpi);
        let target = i32::try_from(target_width).map_err(|_| {
            PageseerError::Pdfium(format!("target width {target_width}px exceeds i32::MAX"))
        })?;
        let cfg = PdfRenderConfig::new().set_target_width(target);
        page.render_with_config(&cfg)
            .map_err(|e| PageseerError::Pdfium(format!("render: {e}")))?
            .as_image()
            .map_err(|e| PageseerError::Pdfium(format!("as_image: {e}")))
    }
}
