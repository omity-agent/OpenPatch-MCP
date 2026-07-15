use super::apply_patch_text;
use std::fs;
#[test]
fn failed_chunk_is_reported_beside_successful_edit() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "one\ntwo\nthree\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", target_path.display()),
        "@@",
        "-one",
        "+1",
        "@@",
        "-missing",
        "+changed",
        "@@",
        "-three",
        "+3",
        "*** End Patch",
        "",
    ]
    .join("\n");
    let result = apply_patch_text(&patch);
    assert!(!result.succeeded);
    assert_eq!(
        result.output,
        format!(
            "<SUCCEEDED>\n<EDIT>\n{}\nbefore: 3 lines, 14 chars\nafter: 3 lines, 8 chars\n</EDIT>\n</SUCCEEDED>\n<FAILED>\n<EDIT>\n{}\n<REASON>\nFailed to find expected lines:\nmissing\n</REASON>\n</EDIT>\n</FAILED>",
            target_path.display(),
            target_path.display()
        )
    );
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "1\ntwo\n3\n");
}
#[test]
fn successful_operations_keep_patch_order_and_specific_stats() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    let added_path = directory.path().join("hello.txt");
    let obsolete_path = directory.path().join("obsolete.txt");
    fs::write(&target_path, "old\n").unwrap();
    fs::write(&obsolete_path, "bye\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", target_path.display()),
        "@@",
        "-old",
        "+new",
        &format!("*** Add File: {}", added_path.display()),
        "+hello",
        "+world",
        &format!("*** Delete File: {}", obsolete_path.display()),
        "*** End Patch",
        "",
    ]
    .join("\n");
    let result = apply_patch_text(&patch);
    assert!(result.succeeded, "{}", result.output);
    assert_eq!(
        result.output,
        format!(
            "<SUCCEEDED>\n<EDIT>\n{}\nbefore: 1 lines, 4 chars\nafter: 1 lines, 4 chars\n</EDIT>\n<ADD>\n{}\nafter: 2 lines, 12 chars\n</ADD>\n<DELETE>\n{}\nbefore: 1 lines, 4 chars\n</DELETE>\n</SUCCEEDED>",
            target_path.display(),
            added_path.display(),
            obsolete_path.display()
        )
    );
}
#[test]
fn parse_failure_has_an_unscoped_reason() {
    let result =
        apply_patch_text("*** Begin Patch\n*** Add File: relative.txt\n+hello\n*** End Patch");
    assert!(!result.succeeded);
    assert_eq!(
        result.output,
        "<FAILED>\n<REASON>\nInvalid patch hunk on line 2: patch paths must be absolute\n</REASON>\n</FAILED>"
    );
}
#[test]
fn empty_patch_uses_the_standard_failure_format() {
    let result = apply_patch_text(" \n");
    assert!(!result.succeeded);
    assert_eq!(
        result.output,
        "<FAILED>\n<REASON>\npatch must not be empty\n</REASON>\n</FAILED>"
    );
}
#[test]
fn failure_reason_is_not_escaped() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "actual\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", target_path.display()),
        "@@",
        "-<expected>&",
        "+new",
        "*** End Patch",
    ]
    .join("\n");
    let result = apply_patch_text(&patch);
    assert!(!result.succeeded);
    assert!(
        result
            .output
            .contains("Failed to find expected lines:\n<expected>&")
    );
}
#[test]
fn move_is_rendered_as_an_edit_of_the_destination() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("old-name.txt");
    let destination_path = directory.path().join("new-name.txt");
    fs::write(&source_path, "content\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", source_path.display()),
        &format!("*** Move to: {}", destination_path.display()),
        "*** End Patch",
    ]
    .join("\n");
    let result = apply_patch_text(&patch);
    assert!(result.succeeded, "{}", result.output);
    assert_eq!(
        result.output,
        format!(
            "<SUCCEEDED>\n<EDIT>\n{}\nbefore: 1 lines, 8 chars\nafter: 1 lines, 8 chars\n</EDIT>\n</SUCCEEDED>",
            destination_path.display()
        )
    );
    assert!(!source_path.exists());
    assert_eq!(fs::read_to_string(destination_path).unwrap(), "content\n");
}
