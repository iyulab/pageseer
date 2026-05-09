//! pageseer `CLI`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use pageseer::{
    extract_with_progress, ImageFormat, Options, PageseerError, SourceInput, StderrProgressSink,
};

/// Document-to-page-image rasterizer.
#[derive(Parser, Debug)]
#[command(name = "pageseer", version, about, long_about = None)]
struct Cli {
    /// 입력 파일 (`PDF`/Office/`HWP`). 1개 이상.
    #[arg(num_args = 1.., required = true)]
    inputs: Vec<PathBuf>,

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

fn summary_to_exit_code(s: &pageseer::BatchSummary) -> u8 {
    if s.pages_failed == 0 && s.documents_failed == 0 {
        0
    } else if s.pages_succeeded > 0 {
        2
    } else {
        1
    }
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
    let inputs: Vec<SourceInput> = cli.inputs.into_iter().map(SourceInput::Path).collect();
    let progress = StderrProgressSink;
    match extract_with_progress(&inputs, opts, &progress) {
        Ok(report) => ExitCode::from(summary_to_exit_code(&report.summary)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use pageseer::BatchSummary;

    fn s(
        succeeded: u32,
        partial: u32,
        failed: u32,
        skipped: u32,
        pages_ok: u32,
        pages_failed: u32,
    ) -> BatchSummary {
        BatchSummary {
            documents_total: succeeded + partial + failed + skipped,
            documents_succeeded: succeeded,
            documents_partial: partial,
            documents_failed: failed,
            documents_skipped: skipped,
            pages_succeeded: pages_ok,
            pages_failed,
        }
    }

    #[test]
    fn all_success_yields_0() {
        assert_eq!(summary_to_exit_code(&s(3, 0, 0, 0, 30, 0)), 0);
    }

    #[test]
    fn partial_with_some_pages_yields_2() {
        assert_eq!(summary_to_exit_code(&s(2, 1, 0, 0, 29, 1)), 2);
    }

    #[test]
    fn all_failed_yields_1() {
        assert_eq!(summary_to_exit_code(&s(0, 0, 3, 0, 0, 0)), 1);
    }

    #[test]
    fn strict_skipped_with_one_failure_yields_1() {
        assert_eq!(summary_to_exit_code(&s(0, 0, 1, 2, 0, 0)), 1);
    }

    #[test]
    fn strict_skipped_with_partial_first_yields_2() {
        assert_eq!(summary_to_exit_code(&s(0, 1, 0, 2, 5, 1)), 2);
    }
}
