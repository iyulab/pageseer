//! `PDF` 엔드투엔드 — 다페이지 fixture → 페이지별 `PNG` 산출.
//!
//! Fixture(`tests/fixtures/sample.pdf`)는 부재 시 `printpdf` dev-dep으로 자동 생성.
//! `PDFium` 라이브러리(`./pdfium/` 또는 시스템) 부재 시 explicit skip.

use std::path::{Path, PathBuf};

use pageseer::{extract, ImageFormat, Options, SourceInput};
use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
};

#[test]
fn three_page_pdf_produces_three_pngs() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    if !fixture.exists() {
        ensure_sample_pdf(&fixture).expect("fixture generation failed");
    }
    assert!(fixture.exists(), "fixture still missing after generate");

    let tmp = tempfile_dir("three-page-pdf");
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 100,
        output_dir: tmp.clone(),
        ..Options::default()
    };
    let report = match extract(SourceInput::Path(fixture.clone()), opts) {
        Ok(r) => r,
        Err(e) => {
            // pdfium 라이브러리 미배치 시 명시적 skip.
            let msg = format!("{e}");
            if msg.contains("library load failed") {
                eprintln!("SKIP: pdfium library not available ({msg})");
                return;
            }
            panic!("extract failed: {e}");
        }
    };

    assert_eq!(report.failed_count(), 0);
    assert_eq!(
        report.succeeded_count(),
        3,
        "expected 3 pages, got {}",
        report.succeeded_count()
    );

    for art in &report.succeeded {
        assert!(
            art.output_path.exists(),
            "missing output: {:?}",
            art.output_path
        );
        let size = std::fs::metadata(&art.output_path).unwrap().len();
        assert!(
            size > 1024,
            "PNG too small ({} bytes): {:?}",
            size,
            art.output_path
        );
    }

    let expected = ["page-001.png", "page-002.png", "page-003.png"];
    for (i, name) in expected.iter().enumerate() {
        assert_eq!(
            report.succeeded[i].output_path.file_name().unwrap(),
            *name,
            "unexpected file name at index {i}"
        );
    }
}

fn ensure_sample_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = build_three_page_pdf();
    std::fs::write(path, bytes)?;
    Ok(())
}

fn build_three_page_pdf() -> Vec<u8> {
    let mut doc = PdfDocument::new("pageseer S1 fixture");
    let pages: Vec<PdfPage> = (1..=3)
        .map(|i| {
            let ops = vec![
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point::new(Mm(20.0), Mm(150.0)),
                },
                Op::SetFont {
                    font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    size: Pt(24.0),
                },
                Op::SetLineHeight { lh: Pt(28.0) },
                Op::ShowText {
                    items: vec![TextItem::Text(format!("pageseer fixture — page {i}"))],
                },
                Op::EndTextSection,
            ];
            // Letter size: 8.5 × 11 inch ≈ 215.9 × 279.4 mm
            PdfPage::new(Mm(215.9), Mm(279.4), ops)
        })
        .collect();
    doc.with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new())
}

fn tempfile_dir(label: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("pageseer-test-{}-{}", std::process::id(), label));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
