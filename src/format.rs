//! 입력 포맷 탐지 — S1 최소판 (확장자만). 매직 바이트 강화는 S5.

use std::path::Path;

/// `extract()`이 수용하는 입력 포맷 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    /// `PDF`.
    Pdf,
    /// `Office` (`DOCX`/`DOC`/`XLSX`/`XLS`/`PPTX`/`PPT`/`ODT`/`ODS`/`ODP`/`RTF`).
    Office,
    /// `HWP`/`HWPX` (한글 문서).
    Hwp,
    /// 그 외 (현재 `UnsupportedFormat` 반환).
    Other,
}

const OFFICE_EXTS: &[&str] = &[
    "docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt", "ods", "odp", "rtf",
];

const HWP_EXTS: &[&str] = &["hwp", "hwpx"];

/// 경로의 확장자만 보고 포맷을 분류한다.
#[must_use]
pub fn detect_from_path(path: &Path) -> DetectedFormat {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("pdf") => DetectedFormat::Pdf,
        Some(e) if OFFICE_EXTS.contains(&e) => DetectedFormat::Office,
        Some(e) if HWP_EXTS.contains(&e) => DetectedFormat::Hwp,
        _ => DetectedFormat::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn pdf_extension_detected() {
        assert_eq!(
            detect_from_path(&PathBuf::from("a.pdf")),
            DetectedFormat::Pdf
        );
        assert_eq!(
            detect_from_path(&PathBuf::from("A.PDF")),
            DetectedFormat::Pdf
        );
    }

    #[test]
    fn unknown_extension_is_other() {
        assert_eq!(
            detect_from_path(&PathBuf::from("a.xyz")),
            DetectedFormat::Other
        );
        assert_eq!(detect_from_path(&PathBuf::from("a")), DetectedFormat::Other);
    }

    #[test]
    fn office_extensions_detected() {
        for ext in &[
            "docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt", "ods", "odp", "rtf", "DOCX",
        ] {
            let p = PathBuf::from(format!("a.{ext}"));
            assert_eq!(detect_from_path(&p), DetectedFormat::Office, "ext={ext}");
        }
    }

    #[test]
    fn hwp_extensions_detected() {
        for ext in &["hwp", "hwpx", "HWP", "HWPX"] {
            let p = PathBuf::from(format!("a.{ext}"));
            assert_eq!(detect_from_path(&p), DetectedFormat::Hwp, "ext={ext}");
        }
    }
}
