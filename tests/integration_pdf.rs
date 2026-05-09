//! `PDF` 엔드투엔드 — 다페이지 fixture → 페이지별 `PNG` 산출.
//!
//! Fixture(`tests/fixtures/sample.pdf`)는 부재 시 `printpdf` dev-dep으로 자동 생성.
//! `PDFium` 라이브러리(`./pdfium/` 또는 시스템) 부재 시 panic — `#[ignore]`로 default
//! `cargo test`에서 제외되며, `cargo test -- --include-ignored`로 명시 실행한다.

use std::path::{Path, PathBuf};

use pageseer::{extract, DocumentOutcome, ImageFormat, Options, SourceInput};
use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
};

mod common;

#[test]
#[ignore = "requires pdfium library at ./pdfium/ or system; run with --include-ignored"]
fn three_page_pdf_produces_three_pngs() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    if !fixture.exists() {
        ensure_sample_pdf(&fixture).expect("fixture generation failed");
    }
    assert!(fixture.exists(), "fixture still missing after generate");

    let tmp = common::tempfile_dir("three-page-pdf");
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 100,
        output_dir: tmp.clone(),
        ..Options::default()
    };
    let report = extract(&[SourceInput::Path(fixture.clone())], opts)
        .expect("extract failed; ensure pdfium library is installed at ./pdfium/");

    assert_eq!(report.summary.documents_total, 1);
    assert_eq!(report.summary.documents_succeeded, 1);
    assert_eq!(report.summary.pages_failed, 0);
    assert_eq!(report.summary.pages_succeeded, 3);

    let inner = match &report.documents[0].outcome {
        DocumentOutcome::Processed(r) => r,
        other => panic!("expected Processed, got {other:?}"),
    };

    for art in &inner.succeeded {
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
            inner.succeeded[i].output_path.file_name().unwrap(),
            *name,
            "unexpected file name at index {i}"
        );
    }
}

#[test]
#[ignore = "requires pdfium library at ./pdfium/ or system; run with --include-ignored"]
fn two_pdfs_with_unique_stems_produce_separate_dirs() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    if !fixture.exists() {
        ensure_sample_pdf(&fixture).expect("fixture generation failed");
    }
    let alt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample-alt.pdf");
    if !alt.exists() {
        std::fs::copy(&fixture, &alt).expect("copy fixture");
    }

    let tmp = common::tempfile_dir("multi-pdf-unique");
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 72,
        output_dir: tmp.clone(),
        ..Options::default()
    };
    let inputs = vec![
        SourceInput::Path(fixture.clone()),
        SourceInput::Path(alt.clone()),
    ];
    let report = extract(&inputs, opts).expect("extract failed");

    assert_eq!(report.summary.documents_total, 2);
    assert_eq!(report.summary.documents_succeeded, 2);
    assert_eq!(report.summary.documents_failed, 0);
    assert_eq!(report.summary.pages_succeeded, 6);
    assert_eq!(report.summary.pages_failed, 0);

    assert!(tmp.join("sample").is_dir(), "expected sample/");
    assert!(tmp.join("sample-alt").is_dir(), "expected sample-alt/");
    assert!(
        !tmp.join("errors.json").exists(),
        "no errors.json on full success"
    );
}

#[test]
#[ignore = "requires pdfium library at ./pdfium/ or system; run with --include-ignored"]
fn two_pdfs_with_colliding_stems_get_dedup_suffix() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    if !fixture.exists() {
        ensure_sample_pdf(&fixture).expect("fixture generation failed");
    }

    let tmp = common::tempfile_dir("multi-pdf-collision");
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 72,
        output_dir: tmp.clone(),
        ..Options::default()
    };
    let inputs = vec![
        SourceInput::Path(fixture.clone()),
        SourceInput::Path(fixture.clone()),
    ];
    let report = extract(&inputs, opts).expect("extract failed");

    assert_eq!(report.summary.documents_total, 2);
    assert_eq!(report.summary.documents_succeeded, 2);
    assert_eq!(report.summary.pages_succeeded, 6);

    assert!(tmp.join("sample").is_dir(), "first input -> sample/");
    assert!(tmp.join("sample-2").is_dir(), "second input -> sample-2/");
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
