//! pageseer `CLI`.

use std::path::PathBuf;
use std::process::ExitCode;

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

    /// 출력 포맷 (S1: png 만 지원).
    #[arg(short = 'f', long = "format", default_value = "png")]
    format: String,

    /// 라스터 `DPI`. 기본 150.
    #[arg(long = "dpi", default_value_t = 150)]
    dpi: u32,

    /// 첫 실패 시 즉시 중단. 기본은 continue-on-error.
    #[arg(long = "strict")]
    strict: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let format = match cli.format.as_str() {
        "png" => ImageFormat::Png,
        other => {
            eprintln!("unsupported --format: {other} (S1: png only)");
            return ExitCode::from(64);
        }
    };
    let opts = Options {
        format,
        dpi: cli.dpi,
        output_dir: cli.output,
        strict: cli.strict,
        ..Options::default()
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
