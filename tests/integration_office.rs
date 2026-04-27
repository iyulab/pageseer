//! Office (`Gotenberg`) 통합 테스트 — `PAGESEER_TEST_GOTENBERG_URL` 환경 변수 + fixture가
//! 모두 있어야 PASS. 어느 하나라도 없으면 explicit skip.
//!
//! 활성화:
//! 1. `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
//! 2. `tests/fixtures/sample.docx` 준비 (사용자 책임)
//! 3. `PAGESEER_TEST_GOTENBERG_URL=http://localhost:3000 cargo test --test integration_office`

use std::path::PathBuf;

use pageseer::{extract, ImageFormat, Options, SourceInput};

#[test]
fn docx_via_gotenberg_produces_pngs() {
    let url = match std::env::var("PAGESEER_TEST_GOTENBERG_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            eprintln!("SKIP: PAGESEER_TEST_GOTENBERG_URL not set");
            return;
        }
    };
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.docx");
    if !fixture.exists() {
        eprintln!("SKIP: tests/fixtures/sample.docx missing");
        return;
    }

    let tmp = tempfile_dir();
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 100,
        output_dir: tmp,
        gotenberg_url: Some(url),
        ..Options::default()
    };
    let report = match extract(SourceInput::Path(fixture.clone()), opts) {
        Ok(r) => r,
        Err(e) => {
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
        assert!(art.output_path.exists());
        assert_eq!(art.source_path.as_deref(), Some(fixture.as_path()));
    }
}

fn tempfile_dir() -> PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("pageseer-office-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
