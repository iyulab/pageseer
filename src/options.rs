//! 공개 Options 타입 — spec §3.2.

use std::path::PathBuf;
use std::time::Duration;

/// 출력 이미지 포맷.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Lossless `PNG`.
    Png,
    /// `JPEG` with quality (1-100).
    Jpeg {
        /// 품질 (1-100).
        quality: u8,
    },
}

impl ImageFormat {
    /// 파일 확장자 반환.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg { .. } => "jpg",
        }
    }
}

/// `extract()` 동작을 제어하는 Options.
#[derive(Debug, Clone)]
pub struct Options {
    /// 출력 이미지 포맷.
    pub format: ImageFormat,
    /// 라스터 `DPI`. 기본 150.
    pub dpi: u32,
    /// 긴 변 최대 픽셀 제한. None이면 무제한.
    pub max_edge: Option<u32>,
    /// 출력 루트 디렉터리.
    pub output_dir: PathBuf,
    /// true = 평면 배치 (`<out>/<stem>-NNN.<ext>`), false = 문서별 하위 디렉터리.
    pub flat: bool,
    /// true = 첫 실패 즉시 중단, false = continue-on-error.
    pub strict: bool,
    /// Gotenberg base `URL`. None이면 env/기본값으로 해석.
    pub gotenberg_url: Option<String>,
    /// Gotenberg 요청 타임아웃.
    pub gotenberg_timeout: Duration,
    /// 문서 단위 병렬도. 0은 1로 clamp.
    pub concurrency: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: ImageFormat::Png,
            dpi: 150,
            max_edge: None,
            output_dir: PathBuf::from("./out"),
            flat: false,
            strict: false,
            gotenberg_url: None,
            gotenberg_timeout: Duration::from_secs(120),
            concurrency: 1,
        }
    }
}

impl Options {
    /// concurrency=0 을 1로 정규화한 복사본.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.concurrency == 0 {
            self.concurrency = 1;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dpi_is_150() {
        assert_eq!(Options::default().dpi, 150);
    }

    #[test]
    fn default_format_is_png() {
        assert_eq!(Options::default().format, ImageFormat::Png);
    }

    #[test]
    fn png_extension_is_png() {
        assert_eq!(ImageFormat::Png.extension(), "png");
    }

    #[test]
    fn jpeg_extension_is_jpg() {
        assert_eq!(ImageFormat::Jpeg { quality: 85 }.extension(), "jpg");
    }

    #[test]
    fn concurrency_zero_is_clamped_to_one() {
        let opts = Options {
            concurrency: 0,
            ..Options::default()
        }
        .normalized();
        assert_eq!(opts.concurrency, 1);
    }

    #[test]
    fn concurrency_positive_is_preserved() {
        let opts = Options {
            concurrency: 8,
            ..Options::default()
        }
        .normalized();
        assert_eq!(opts.concurrency, 8);
    }
}
