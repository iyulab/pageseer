//! pageseer `CLI`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use pageseer::{extract, ImageFormat, Options, PageseerError, SourceInput};

/// Document-to-page-image rasterizer.
#[derive(Parser, Debug)]
#[command(name = "pageseer", version, about, long_about = None)]
struct Cli {
    /// 입력 파일.
    input: PathBuf,

    /// 출력 디렉터리. 기본 `./out`.
    #[arg(short = 'o', long = "output", default_value = "./out")]
    output: PathBuf,

    /// 출력 포맷 (`png` 또는 `jpeg`).
    #[arg(short = 'f', long = "format", default_value = "png")]
    format: String,

    /// `JPEG` 품질 (1-100). format=png일 때 무시.
    #[arg(short = 'q', long = "quality", default_value_t = 85, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,

    /// 라스터 `DPI`. 기본 150.
    #[arg(long = "dpi", default_value_t = 150)]
    dpi: u32,

    /// 긴 변 최대 픽셀(라스터 후 다운스케일). 미지정 시 무제한.
    #[arg(long = "max-edge")]
    max_edge: Option<u32>,

    /// 첫 실패 시 즉시 중단. 기본은 continue-on-error.
    #[arg(long = "strict")]
    strict: bool,

    /// 평면 배치 (`<out>/<stem>-NNN.<ext>`). 기본은 문서별 하위 디렉터리.
    #[arg(long = "flat")]
    flat: bool,

    /// 문서 단위 병렬도.
    #[arg(short = 'j', long = "concurrency", default_value_t = 1)]
    concurrency: usize,

    /// Gotenberg base `URL`. 미지정시 `GOTENBERG_URL` env, 그것도 없으면 `http://localhost:3000`.
    #[arg(long = "gotenberg-url")]
    gotenberg_url: Option<String>,

    /// Gotenberg 요청 타임아웃(초). 기본 120.
    #[arg(long = "gotenberg-timeout", default_value_t = 120)]
    gotenberg_timeout: u64,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = match cli.format.as_str() {
        "png" => ImageFormat::Png,
        "jpeg" => ImageFormat::Jpeg {
            quality: cli.quality,
        },
        other => {
            eprintln!("unsupported --format: {other} (allowed: png, jpeg)");
            return ExitCode::from(64);
        }
    };
    let opts = Options {
        format,
        dpi: cli.dpi,
        max_edge: cli.max_edge,
        output_dir: cli.output,
        flat: cli.flat,
        strict: cli.strict,
        gotenberg_url: cli.gotenberg_url,
        gotenberg_timeout: Duration::from_secs(cli.gotenberg_timeout),
        concurrency: cli.concurrency,
    };
    let inputs = vec![SourceInput::Path(cli.input)];
    match extract(&inputs, opts) {
        Ok(report) => {
            let s = report.summary;
            if s.pages_failed == 0 && s.documents_failed == 0 {
                eprintln!(
                    "pageseer: {} pages OK across {} documents",
                    s.pages_succeeded, s.documents_total
                );
                ExitCode::from(0)
            } else if s.pages_succeeded > 0 {
                eprintln!(
                    "pageseer: {}/{} pages OK ({} failed) — see errors.json",
                    s.pages_succeeded,
                    s.pages_succeeded + s.pages_failed,
                    s.pages_failed + s.documents_failed
                );
                ExitCode::from(2)
            } else {
                eprintln!(
                    "pageseer: all failed ({} documents failed) — see errors.json",
                    s.documents_failed
                );
                ExitCode::from(1)
            }
        }
        Err(PageseerError::Config(msg)) => {
            eprintln!("pageseer: config error: {msg}");
            ExitCode::from(64)
        }
        Err(e) => {
            eprintln!("pageseer error: {e}");
            ExitCode::from(1)
        }
    }
}
