//! 출력 경로 계산 — spec §4.4.

use std::path::{Path, PathBuf};

use crate::options::ImageFormat;

/// 페이지 번호 패딩 폭 — 페이지 수에 따라 결정 (최소 3자리).
#[must_use]
pub fn padding_width(page_count: usize) -> usize {
    let mut n = page_count.max(1);
    let mut digits = 0_usize;
    while n > 0 {
        digits += 1;
        n /= 10;
    }
    digits.max(3)
}

/// 단일 페이지 산출물 경로 계산.
///
/// flat=false: `<out>/<stem>/page-NNN.<ext>`
/// flat=true:  `<out>/<stem>-NNN.<ext>` (S5에서 활성. S1은 flat=false만 사용)
#[must_use]
pub fn page_output_path(
    output_dir: &Path,
    source_stem: &str,
    page_index_zero_based: u32,
    page_count: usize,
    format: ImageFormat,
    flat: bool,
) -> PathBuf {
    let pad = padding_width(page_count);
    let one_based = page_index_zero_based + 1;
    let ext = format.extension();
    if flat {
        let filename = format!("{source_stem}-{one_based:0pad$}.{ext}");
        output_dir.join(filename)
    } else {
        let filename = format!("page-{one_based:0pad$}.{ext}");
        output_dir.join(source_stem).join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn padding_minimum_is_three() {
        assert_eq!(padding_width(1), 3);
        assert_eq!(padding_width(99), 3);
        assert_eq!(padding_width(999), 3);
    }

    #[test]
    fn padding_grows_for_large_documents() {
        assert_eq!(padding_width(1000), 4);
        assert_eq!(padding_width(99999), 5);
    }

    #[test]
    fn nested_path_default() {
        let p = page_output_path(
            &PathBuf::from("./out"),
            "report",
            0,
            3,
            ImageFormat::Png,
            false,
        );
        assert_eq!(
            p,
            PathBuf::from("./out").join("report").join("page-001.png")
        );
    }

    #[test]
    fn flat_path_uses_stem_prefix() {
        let p = page_output_path(
            &PathBuf::from("./out"),
            "report",
            2,
            10,
            ImageFormat::Jpeg { quality: 85 },
            true,
        );
        assert_eq!(p, PathBuf::from("./out").join("report-003.jpg"));
    }

    #[test]
    fn three_digit_padding_for_under_1000_pages() {
        let p = page_output_path(
            &PathBuf::from("./out"),
            "doc",
            42,
            500,
            ImageFormat::Png,
            false,
        );
        assert_eq!(p.file_name().unwrap(), "page-043.png");
    }
}
