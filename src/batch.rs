//! 배치 orchestration 헬퍼 — spec §1.

use std::path::{Path, PathBuf};

use crate::error::PageseerError;
use crate::report::BatchSummary;
use crate::SourceInput;

/// 배치 진행 상황 콜백. 모든 메서드는 동시 스레드에서 호출될 수 있다.
pub trait ProgressSink: Send + Sync {
    /// 문서 처리 시작.
    fn record_start(&self, input_index: u32, total: u32, label: &str);
    /// 문서 처리 완료 (페이지 단계 도달).
    fn record_done(
        &self,
        input_index: u32,
        total: u32,
        label: &str,
        pages_ok: u32,
        pages_failed: u32,
    );
    /// 문서 단위 치명 실패 (`SourceRead` / `Convert`).
    fn record_failed(&self, input_index: u32, total: u32, label: &str, message: &str);
    /// strict cancel로 시도 안 함.
    fn record_skipped(&self, input_index: u32, total: u32, label: &str);
    /// 배치 종료 — 마지막 1줄 요약.
    fn record_summary(&self, summary: &BatchSummary);
}

/// 모든 이벤트를 무시하는 sink — 라이브러리 사용자나 테스트용.
pub struct NullProgressSink;

impl ProgressSink for NullProgressSink {
    fn record_start(&self, _: u32, _: u32, _: &str) {}
    fn record_done(&self, _: u32, _: u32, _: &str, _: u32, _: u32) {}
    fn record_failed(&self, _: u32, _: u32, _: &str, _: &str) {}
    fn record_skipped(&self, _: u32, _: u32, _: &str) {}
    fn record_summary(&self, _: &BatchSummary) {}
}

/// stderr 기반 sink — `pageseer:` prefix + 한 줄/이벤트.
pub struct StderrProgressSink;

impl ProgressSink for StderrProgressSink {
    fn record_start(&self, idx: u32, total: u32, label: &str) {
        eprintln!("pageseer: [{}/{}] processing {}...", idx + 1, total, label);
    }
    fn record_done(&self, idx: u32, total: u32, label: &str, ok: u32, failed: u32) {
        if failed == 0 {
            eprintln!(
                "pageseer: [{}/{}] {} done ({} pages)",
                idx + 1,
                total,
                label,
                ok
            );
        } else {
            eprintln!(
                "pageseer: [{}/{}] {} done ({} pages, {} failed)",
                idx + 1,
                total,
                label,
                ok,
                failed
            );
        }
    }
    fn record_failed(&self, idx: u32, total: u32, label: &str, msg: &str) {
        eprintln!(
            "pageseer: [{}/{}] {} FAILED: {}",
            idx + 1,
            total,
            label,
            msg
        );
    }
    fn record_skipped(&self, idx: u32, total: u32, label: &str) {
        eprintln!(
            "pageseer: [{}/{}] {} skipped (--strict)",
            idx + 1,
            total,
            label
        );
    }
    fn record_summary(&self, s: &BatchSummary) {
        let total_pages = s.pages_succeeded + s.pages_failed;
        let see_errors = if s.pages_failed > 0 || s.documents_failed > 0 {
            " — see errors.json"
        } else {
            ""
        };
        eprintln!(
            "pageseer: {}/{} pages OK across {} documents ({} partial, {} failed, {} skipped){}",
            s.pages_succeeded,
            total_pages,
            s.documents_total,
            s.documents_partial,
            s.documents_failed,
            s.documents_skipped,
            see_errors
        );
    }
}

/// 진행 로그/에러 노출용 표시 라벨. `Path` 입력은 `file_name`, `Bytes`는 filename hint.
#[must_use]
pub(crate) fn source_label(input: &SourceInput) -> String {
    match input {
        SourceInput::Path(p) => p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<no-name>")
            .to_owned(),
        SourceInput::Bytes { filename, .. } => filename.clone(),
    }
}

fn stem_of(input: &SourceInput) -> String {
    match input {
        SourceInput::Path(p) => p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_owned(),
        SourceInput::Bytes { filename, .. } => Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_owned(),
    }
}

/// 입력별 출력 디렉터리를 결정한다.
///
/// `flat=false`: `<output_dir>/<stem>/` (충돌 시 `<stem>-2/`, `<stem>-3/`, ...).
/// `flat=true`:  `<output_dir>` 자체. stem 충돌이 있으면 `Err(Config)`.
pub(crate) fn dedup_output_dirs(
    inputs: &[SourceInput],
    output_dir: &Path,
    flat: bool,
) -> Result<Vec<PathBuf>, PageseerError> {
    let stems: Vec<String> = inputs.iter().map(stem_of).collect();

    if flat {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for stem in &stems {
            if !seen.insert(stem.as_str()) {
                return Err(PageseerError::Config(format!(
                    "stem collision in --flat mode: {stem:?} appears more than once"
                )));
            }
        }
        return Ok(vec![output_dir.to_path_buf(); inputs.len()]);
    }

    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut result = Vec::with_capacity(inputs.len());
    for stem in &stems {
        let n = counts.entry(stem.clone()).or_insert(0);
        *n += 1;
        let dir_name = if *n == 1 {
            stem.clone()
        } else {
            format!("{stem}-{n}")
        };
        result.push(output_dir.join(dir_name));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::BatchSummary;

    fn path_input(name: &str) -> SourceInput {
        SourceInput::Path(PathBuf::from(name))
    }

    #[test]
    fn unique_stems_pass_through() {
        let inputs = vec![
            path_input("a.pdf"),
            path_input("b.pdf"),
            path_input("c.pdf"),
        ];
        let dirs = dedup_output_dirs(&inputs, Path::new("./out"), false).unwrap();
        assert_eq!(dirs[0], PathBuf::from("./out").join("a"));
        assert_eq!(dirs[1], PathBuf::from("./out").join("b"));
        assert_eq!(dirs[2], PathBuf::from("./out").join("c"));
    }

    #[test]
    fn collision_yields_suffix_in_input_order() {
        let inputs = vec![
            path_input("dir1/file.pdf"),
            path_input("dir2/file.pdf"),
            path_input("dir3/file.pdf"),
        ];
        let dirs = dedup_output_dirs(&inputs, Path::new("./out"), false).unwrap();
        assert_eq!(dirs[0], PathBuf::from("./out").join("file"));
        assert_eq!(dirs[1], PathBuf::from("./out").join("file-2"));
        assert_eq!(dirs[2], PathBuf::from("./out").join("file-3"));
    }

    #[test]
    fn flat_mode_unique_returns_output_dir_for_all() {
        let inputs = vec![path_input("a.pdf"), path_input("b.pdf")];
        let dirs = dedup_output_dirs(&inputs, Path::new("./out"), true).unwrap();
        assert_eq!(dirs[0], PathBuf::from("./out"));
        assert_eq!(dirs[1], PathBuf::from("./out"));
    }

    #[test]
    fn flat_mode_collision_is_config_error() {
        let inputs = vec![path_input("a/x.pdf"), path_input("b/x.pdf")];
        let err = dedup_output_dirs(&inputs, Path::new("./out"), true).unwrap_err();
        match err {
            PageseerError::Config(msg) => assert!(msg.contains("stem collision")),
            other => panic!("expected Config, got {other:?}"),
        }
    }

    #[test]
    fn bytes_input_uses_filename_stem() {
        let inputs = vec![SourceInput::Bytes {
            data: vec![],
            filename: "report.docx".into(),
        }];
        let dirs = dedup_output_dirs(&inputs, Path::new("./out"), false).unwrap();
        assert_eq!(dirs[0], PathBuf::from("./out").join("report"));
    }

    #[test]
    fn null_sink_drops_events() {
        let sink = NullProgressSink;
        sink.record_start(0, 3, "a.pdf");
        sink.record_done(0, 3, "a.pdf", 5, 0);
        sink.record_failed(0, 3, "a.pdf", "boom");
        sink.record_skipped(0, 3, "a.pdf");
        sink.record_summary(&BatchSummary {
            documents_total: 3,
            ..BatchSummary::default()
        });
    }

    #[test]
    fn captured_sink_records_events_in_order() {
        use std::sync::Mutex;
        struct Cap<'a>(&'a Mutex<Vec<String>>);
        impl ProgressSink for Cap<'_> {
            fn record_start(&self, idx: u32, total: u32, label: &str) {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("start {idx}/{total} {label}"));
            }
            fn record_done(&self, idx: u32, total: u32, label: &str, ok: u32, fail: u32) {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("done {idx}/{total} {label} {ok}/{fail}"));
            }
            fn record_failed(&self, idx: u32, total: u32, label: &str, msg: &str) {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("failed {idx}/{total} {label} {msg}"));
            }
            fn record_skipped(&self, idx: u32, total: u32, label: &str) {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("skipped {idx}/{total} {label}"));
            }
            fn record_summary(&self, _s: &BatchSummary) {}
        }
        let captured: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let sink = Cap(&captured);
        sink.record_start(0, 2, "x");
        sink.record_done(0, 2, "x", 3, 0);
        sink.record_failed(1, 2, "y", "bad");
        let lines = captured.lock().unwrap();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("start 0/2 x"));
        assert!(lines[1].contains("done 0/2 x 3/0"));
        assert!(lines[2].contains("failed 1/2 y bad"));
    }
}
