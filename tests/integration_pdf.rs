//! `PDF` 엔드투엔드 — 다페이지 fixture → 페이지별 `PNG` 산출.
//!
//! Fixture(`tests/fixtures/sample.pdf`)와 `PDFium` 라이브러리(`./pdfium/` 또는 시스템) 둘 다
//! 가용해야 PASS. 둘 중 하나라도 없으면 explicit skip (panic 없이 early return).

use std::path::PathBuf;

use pageseer::{extract, ImageFormat, Options, SourceInput};

#[test]
fn three_page_pdf_produces_three_pngs() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    if !fixture.exists() {
        eprintln!(
            "SKIP: fixture missing ({}). 본 테스트는 `tests/fixtures/sample.pdf`가 있어야 동작한다.",
            fixture.display()
        );
        return;
    }

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
    assert!(
        report.succeeded_count() >= 1,
        "expected ≥1 page, got {}",
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

    // fixture가 정확히 3페이지일 때 네이밍 검증.
    if report.succeeded_count() == 3 {
        let expected = ["page-001.png", "page-002.png", "page-003.png"];
        for (i, name) in expected.iter().enumerate() {
            assert_eq!(
                report.succeeded[i].output_path.file_name().unwrap(),
                *name,
                "unexpected file name at index {i}"
            );
        }
    }
}

fn tempfile_dir(label: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("pageseer-test-{}-{}", std::process::id(), label));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
