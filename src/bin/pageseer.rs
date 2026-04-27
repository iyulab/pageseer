//! pageseer `CLI`.
//!
//! S0 bootstrap: `--version`과 `--help`만 동작. 실제 처리는 S1+에서 추가.

use clap::Parser;

/// Document-to-page-image rasterizer.
#[derive(Parser, Debug)]
#[command(name = "pageseer", version, about, long_about = None)]
struct Cli {
    /// 입력 파일 경로 (1개 이상). S0에서는 무시됨.
    #[arg(required = false)]
    inputs: Vec<String>,
}

fn main() {
    let _cli = Cli::parse();
    eprintln!(
        "pageseer {} — S0 bootstrap; document processing not yet implemented",
        env!("CARGO_PKG_VERSION")
    );
    std::process::exit(64);
}
