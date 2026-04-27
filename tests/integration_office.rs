//! Office (`Gotenberg`) 통합 테스트 — `#[ignore]`로 default `cargo test`에서 제외된다.
//!
//! 활성화:
//! 1. `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`
//! 2. `PAGESEER_TEST_GOTENBERG_URL=http://localhost:3000 cargo test --test integration_office -- --include-ignored`
//!
//! `tests/fixtures/sample.docx`는 부재 시 `docx-rs` dev-dep으로 자동 생성된다.
//! env/pdfium 부재 시는 panic — silent SKIP 안티패턴 회피 (S3.5 표준).

use std::path::{Path, PathBuf};

use docx_rs::{Docx, Paragraph, Run};
use pageseer::{extract, ImageFormat, Options, SourceInput};

mod common;

#[test]
#[ignore = "requires Gotenberg server + pdfium; run with --include-ignored"]
fn docx_via_gotenberg_produces_pngs() {
    let url = std::env::var("PAGESEER_TEST_GOTENBERG_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .expect(
            "PAGESEER_TEST_GOTENBERG_URL not set; set to e.g. http://localhost:3000 \
             after starting `docker run --rm -p 3000:3000 gotenberg/gotenberg:8`",
        );
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.docx");
    if !fixture.exists() {
        ensure_sample_docx(&fixture).expect("fixture generation failed");
    }
    assert!(fixture.exists(), "fixture still missing after generate");

    let tmp = common::tempfile_dir("office");
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
        "expected >=1 page, got {}",
        report.succeeded_count()
    );
    for art in &report.succeeded {
        assert!(art.output_path.exists());
        assert_eq!(art.source_path.as_deref(), Some(fixture.as_path()));
    }
}

fn ensure_sample_docx(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(path)?;
    Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("pageseer S6 fixture")))
        .build()
        .pack(file)?;
    Ok(())
}
