//! `HWP`/`HWPX` 엔드투엔드 — fixture → 페이지별 `PNG`.
//!
//! 활성화 (S4 표준):
//! 1. `tests/fixtures/sample.hwp` 직접 배치 (rhwp는 디코더 전용 — 자동 생성 불가)
//! 2. 한글 폰트 설치 권장 (Linux=Noto Sans CJK KR, Windows=맑은 고딕, macOS=Apple SD Gothic Neo)
//! 3. `pdfium` 라이브러리 (`./pdfium/` 또는 시스템) 설치
//! 4. `cargo test --test integration_hwp -- --include-ignored`
//!
//! fixture/lib 부재 시 `panic` — silent SKIP 안티패턴 회피 (S3.5 표준).

use std::path::PathBuf;

use pageseer::{extract, ImageFormat, Options, SourceInput};

mod common;

#[test]
#[ignore = "requires sample.hwp + pdfium + Korean font; run with --include-ignored"]
fn hwp_sample_produces_pages() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.hwp");
    assert!(
        fixture.exists(),
        "fixture missing: {fixture:?}; place a .hwp file at tests/fixtures/sample.hwp \
         (rhwp is decoder-only — cannot auto-generate). See README §HWP."
    );

    let tmp = common::tempfile_dir("hwp");
    let opts = Options {
        format: ImageFormat::Png,
        dpi: 100,
        output_dir: tmp,
        ..Options::default()
    };
    let report = extract(SourceInput::Path(fixture.clone()), opts).expect(
        "extract failed; ensure pdfium library is installed at ./pdfium/ \
         and the hwp parses successfully",
    );

    assert_eq!(report.failed_count(), 0);
    assert!(
        report.succeeded_count() >= 1,
        "expected ≥1 page, got {}",
        report.succeeded_count()
    );
    for art in &report.succeeded {
        assert!(art.output_path.exists(), "missing: {:?}", art.output_path);
        assert_eq!(
            art.source_path.as_deref(),
            Some(fixture.as_path()),
            "source_path should be the original .hwp path, not tmp pdf"
        );
        let size = std::fs::metadata(&art.output_path).unwrap().len();
        assert!(size > 1024, "PNG too small ({size} bytes)");
    }
}
