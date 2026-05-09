//! 공개 에러 타입 — spec §3.3.

use thiserror::Error;

/// pageseer 공개 `API`의 루트 에러.
#[derive(Debug, Error)]
pub enum PageseerError {
    /// 파일 I/O 실패.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Gotenberg `HTTP` 오류.
    #[error("gotenberg error (status {status:?}, trace {trace:?}): {message}")]
    Gotenberg {
        /// `HTTP` 상태 코드 (연결 실패면 None).
        status: Option<u16>,
        /// `Gotenberg-Trace` 헤더.
        trace: Option<String>,
        /// 사람이 읽을 수 있는 설명.
        message: String,
    },

    /// `PDFium` 렌더 실패.
    #[error("pdfium error: {0}")]
    Pdfium(String),

    /// rhwp `HWP` 처리 실패.
    #[error("rhwp error: {0}")]
    Rhwp(String),

    /// 설정 오류 (인자 불일치, 빈 입력, 풀 빌드 실패 등).
    #[error("config error: {0}")]
    Config(String),
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
    fn rhwp_displays_underlying_message() {
        let err = PageseerError::Rhwp("parse hwp: bad magic".to_owned());
        let msg = format!("{err}");
        assert!(msg.contains("rhwp"), "message: {msg}");
        assert!(msg.contains("bad magic"), "message: {msg}");
    }

    #[test]
    fn config_displays_underlying_message() {
        let err = PageseerError::Config("empty input list".to_owned());
        let msg = format!("{err}");
        assert!(msg.contains("empty input list"), "message: {msg}");
    }
}
