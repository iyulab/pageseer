//! 배치 orchestration 헬퍼 — spec §1.

// Task 7에서 lib.rs가 이 함수들을 호출하기 전까지 일시적으로 허용.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::error::PageseerError;
use crate::SourceInput;

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
}
