use super::super::{apply::files, model::FileState, store::Transition};
use std::fs;
#[test]
fn exact_validation_accepts_large_unicode_contents() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unicode.txt");
    let original = format!("{}é-tail", "a".repeat(32 * 1_024));
    fs::write(&path, &original).unwrap();
    let observed = files::snapshot(&path, "snapshot failed").unwrap();
    let transitions = [Transition {
        path: path.clone(),
        before: observed,
        after: FileState::present(String::from("updated")),
    }];
    files::verify(&transitions).unwrap();
    files::publish(&transitions).unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "updated");
}
#[test]
fn exact_validation_rejects_same_length_concurrent_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("same-length.txt");
    let original = "a".repeat(32 * 1_024);
    let concurrent = "b".repeat(original.len());
    fs::write(&path, &original).unwrap();
    let observed = files::snapshot(&path, "snapshot failed").unwrap();
    let transitions = [Transition {
        path: path.clone(),
        before: observed,
        after: FileState::present(String::from("updated")),
    }];
    fs::write(&path, &concurrent).unwrap();
    let error = files::verify(&transitions).unwrap_err();
    assert!(error.to_string().contains("file changed concurrently"));
    assert_eq!(fs::read_to_string(path).unwrap(), concurrent);
}
#[test]
fn exact_validation_rejects_a_file_created_at_a_missing_path() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("created.txt");
    let observed = FileState::Missing;
    let transitions = [Transition {
        path: path.clone(),
        before: observed,
        after: FileState::present(String::from("planned")),
    }];
    fs::write(&path, "concurrent").unwrap();
    let error = files::verify(&transitions).unwrap_err();
    assert!(error.to_string().contains("file changed concurrently"));
    assert_eq!(fs::read_to_string(path).unwrap(), "concurrent");
}
