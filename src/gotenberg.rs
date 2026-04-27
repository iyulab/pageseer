//! Gotenberg `HTTP` 어댑터 — spec §5.2.

use std::path::Path;
use std::time::Duration;

use reqwest::blocking::{multipart, Client};

use crate::error::PageseerError;

/// Gotenberg base `URL`을 해석한다 (옵션 → env → 기본).
#[must_use]
pub fn resolve_base_url(option_url: Option<&str>) -> String {
    if let Some(u) = option_url {
        return u.trim_end_matches('/').to_owned();
    }
    if let Ok(u) = std::env::var("GOTENBERG_URL") {
        return u.trim_end_matches('/').to_owned();
    }
    "http://localhost:3000".to_owned()
}

/// Gotenberg 원격 호출 어댑터 (`LibreOffice` 변환 한정).
pub struct GotenbergClient {
    base_url: String,
    http: Client,
}

impl GotenbergClient {
    /// 새 클라이언트. `timeout`은 connect+read 통합.
    pub fn new(base_url: String, timeout: Duration) -> Result<Self, PageseerError> {
        let http =
            Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| PageseerError::Gotenberg {
                    status: None,
                    trace: None,
                    message: format!("client build: {e}"),
                })?;
        Ok(Self { base_url, http })
    }

    /// Office 파일 1개 → `PDF` bytes.
    ///
    /// 다파일 처리(`ZIP` 응답)는 v0.1 비목표. 단일 입력 한정.
    pub fn convert_office(&self, path: &Path) -> Result<Vec<u8>, PageseerError> {
        let url = format!("{}/forms/libreoffice/convert", self.base_url);
        let part = multipart::Part::file(path).map_err(|e| PageseerError::Gotenberg {
            status: None,
            trace: None,
            message: format!("read input file: {e}"),
        })?;
        let form = multipart::Form::new().part("files", part);
        let response =
            self.http
                .post(&url)
                .multipart(form)
                .send()
                .map_err(|e| PageseerError::Gotenberg {
                    status: None,
                    trace: None,
                    message: format!("send: {e}"),
                })?;
        let trace = response
            .headers()
            .get("Gotenberg-Trace")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(PageseerError::Gotenberg {
                status: Some(status.as_u16()),
                trace,
                message: format!("HTTP {status}: {body}"),
            });
        }
        let bytes = response.bytes().map_err(|e| PageseerError::Gotenberg {
            status: Some(status.as_u16()),
            trace: trace.clone(),
            message: format!("read body: {e}"),
        })?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_base_url_explicit_takes_precedence() {
        let url = resolve_base_url(Some("http://example.com/"));
        assert_eq!(url, "http://example.com");
    }

    #[test]
    fn resolve_base_url_trims_trailing_slash() {
        let url = resolve_base_url(Some("http://example.com////"));
        assert_eq!(url, "http://example.com");
    }
}
