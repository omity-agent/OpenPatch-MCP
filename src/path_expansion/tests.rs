use super::{VariableValue, expand_with};
use std::path::PathBuf;
#[test]
fn expands_unix_and_windows_variables() {
    let expanded = expand_with(
        "$ROOT/${LEAF}/%FILE%",
        |name| match name {
            "ROOT" => VariableValue::Present(String::from("base")),
            "LEAF" => VariableValue::Present(String::from("dir")),
            "FILE" => VariableValue::Present(String::from("target.txt")),
            _ => VariableValue::Missing,
        },
        || None,
    )
    .unwrap();
    assert_eq!(expanded, "base/dir/target.txt");
}
#[test]
fn expands_home_prefix() {
    let expanded = expand_with(
        "~/target.txt",
        |_| VariableValue::Missing,
        || Some(PathBuf::from("/home/user")),
    )
    .unwrap();
    assert_eq!(expanded, "/home/user/target.txt");
}
#[test]
fn expands_backslash_home_prefix() {
    let expanded = expand_with(
        "~\\target.txt",
        |_| VariableValue::Missing,
        || Some(PathBuf::from("C:\\Users\\agent")),
    )
    .unwrap();
    assert_eq!(expanded, "C:\\Users\\agent\\target.txt");
}
#[test]
fn reports_missing_variable() {
    let error =
        expand_with("$MISSING/target.txt", |_| VariableValue::Missing, || None).unwrap_err();
    assert_eq!(
        error.to_string(),
        "environment variable 'MISSING' is not set in path '$MISSING/target.txt'"
    );
}
#[test]
fn leaves_literal_percent_text_unchanged() {
    let expanded = expand_with("100%/target.txt", |_| VariableValue::Missing, || None).unwrap();
    assert_eq!(expanded, "100%/target.txt");
}
