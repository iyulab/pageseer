//! 공개 에러 타입 — spec §3.3.

use std::path::PathBuf;

use thiserror::Error;

/// pageseer 공개 `API`의 루트 에러.
#[derive(Debug, Error)]
pub enum PageseerError {
    /// 파일 I/O 실패.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Gotenberg `HTTP` 오류 (S3에서 사용).
    #[error("gotenberg error (status {status:?}, trace {trace:?}): {message}")]
    Gotenberg {
        /// `HTTP` 상태 코드 (연결 실패면 None).
        status: Option<u16>,
        /// `Gotenberg-Trace` 헤더.
        trace: Option<String>,
        /// 사람이 읽을 수 있는 설명.
        message: String,
    },

    /// `PDFium` 렌더 실패 (S1+).
    #[error("pdfium error: {0}")]
    Pdfium(String),

    /// rhwp `HWP` 처리 실패 (S4).
    #[error("rhwp error: {0}")]
    Rhwp(String),

    /// 알 수 없는/미지원 입력 포맷.
    #[error("unsupported format (ext={extension}): {path:?}")]
    UnsupportedFormat {
        /// 감지된 확장자 또는 매직 표식.
        extension: String,
        /// 입력 경로 (bytes 입력이면 None).
        path: Option<PathBuf>,
    },

    /// 설정 오류 (인자 불일치 등).
    #[error("config error: {0}")]
    Config(String),

    /// 부분 실패 — strict=false 완료 중 일부 실패. `report`에 성공·실패 내역.
    #[error("partial failure: {} ok, {} failed", .0.succeeded_count(), .0.failed_count())]
    Partial(crate::report::ExtractReport),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_displays_underlying_message() {
        let err = PageseerError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        let msg = format!("{err}");
        assert!(msg.contains("missing"), "message: {msg}");
    }

    #[test]
    fn unsupported_format_displays_extension() {
        let err = PageseerError::UnsupportedFormat {
            extension: "xyz".to_owned(),
            path: None,
        };
        let msg = format!("{err}");
        assert!(msg.contains("xyz"), "message: {msg}");
    }
}
