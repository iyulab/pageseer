//! Office (`Gotenberg`) 통합 테스트 — `#[ignore]`로 default `cargo test`에서 제외된다.
//!
//! 활성화:
//! 1. `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
//! 2. `tests/fixtures/sample.docx` 준비 (사용자 책임 — Pending #2)
//! 3. `PAGESEER_TEST_GOTENBERG_URL=http://localhost:3000 cargo test --test integration_office -- --include-ignored`
//!
//! pdfium 라이브러리 부재 시 panic. env/fixture 부재 시 explicit skip (별도 opt-in gate).

use std::path::PathBuf;

use pageseer::{extract, ImageFormat, Options, SourceInput};

#[test]
#[ignore = "requires Gotenberg server + pdfium; run with --include-ignored"]
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
    let report = extract(SourceInput::Path(fixture.clone()), opts)
        .expect("extract failed; ensure pdfium library is installed at ./pdfium/");
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
