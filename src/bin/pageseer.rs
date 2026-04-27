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
    /// 입력 `PDF` 파일.
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

    /// 문서 단위 병렬도. v0.1은 단일 입력만 지원하므로 효과 없음 (multi-input은 v0.2).
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
    match extract(SourceInput::Path(cli.input), opts) {
        Ok(report) => {
            eprintln!("pageseer: {} pages OK, 0 failed", report.succeeded_count());
            ExitCode::from(0)
        }
        Err(PageseerError::Partial(report)) => {
            eprintln!(
                "pageseer: {} pages OK, {} failed (see errors.json)",
                report.succeeded_count(),
                report.failed_count()
            );
            ExitCode::from(2)
        }
        Err(e @ (PageseerError::Config(_) | PageseerError::UnsupportedFormat { .. })) => {
            eprintln!("pageseer: {e}");
            ExitCode::from(64)
        }
        Err(e) => {
            eprintln!("pageseer error: {e}");
            ExitCode::from(1)
        }
    }
}
