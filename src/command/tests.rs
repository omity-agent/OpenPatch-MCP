use super::{PatchExecution, PatchRunner};
use std::fs;
#[test]
fn multiline_patch_is_applied_without_executable() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "old\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", target_path.display()),
        "@@",
        "-old",
        "+new",
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "new\n");
    assert_eq!(
        output.render(),
        format!(
            "<SUCCEEDED>\n<EDIT>\n{}\nbefore: 1 lines, 4 chars\nafter: 1 lines, 4 chars\n</EDIT>\n</SUCCEEDED>",
            target_path.display()
        )
    );
}
#[test]
fn failed_update_does_not_stop_following_files() {
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("a.txt");
    let second_path = directory.path().join("b.txt");
    let third_path = directory.path().join("c.txt");
    fs::write(&first_path, "old\n").unwrap();
    fs::write(&second_path, "kept\n").unwrap();
    fs::write(&third_path, "old\n").unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", first_path.display()),
        "@@",
        "-old",
        "+new",
        &format!("*** Update File: {}", second_path.display()),
        "@@",
        "-kepx",
        "+changed",
        &format!("*** Update File: {}", third_path.display()),
        "@@",
        "-old",
        "+new",
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(!output.succeeded());
    assert_eq!(fs::read_to_string(&first_path).unwrap(), "new\n");
    assert_eq!(fs::read_to_string(&second_path).unwrap(), "kept\n");
    assert_eq!(fs::read_to_string(&third_path).unwrap(), "new\n");
    assert!(output.render().contains("<SUCCEEDED>"));
    assert!(output.render().contains("<FAILED>"));
    assert!(output.render().contains(&second_path.display().to_string()));
    assert!(
        output
            .render()
            .contains("Failed to find expected lines. Closest match:\n```\nkept\n```")
    );
}
#[test]
fn delete_missing_file_reports_delete_context() {
    let directory = tempfile::tempdir().unwrap();
    let missing_path = directory.path().join("missing.txt");
    let patch = [
        "*** Begin Patch",
        &format!("*** Delete File: {}", missing_path.display()),
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(!output.succeeded());
    assert!(output.render().contains("<DELETE>"));
    assert!(
        output
            .render()
            .contains(&missing_path.display().to_string())
    );
    assert!(output.render().contains("Failed to delete file:"));
}
#[test]
fn delete_directory_reports_reference_style_context() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target");
    fs::create_dir_all(&target_path).unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Delete File: {}", target_path.display()),
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(!output.succeeded());
    assert!(output.render().contains("Failed to delete file:"));
    assert!(output.render().contains("path is a directory"));
}
#[test]
fn relative_patch_path_is_an_unscoped_failure() {
    let patch = [
        "*** Begin Patch",
        "*** Add File: relative.txt",
        "+hello",
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(!output.succeeded());
    assert_eq!(
        output.render(),
        "<FAILED>\n<REASON>\nInvalid patch hunk on line 2: patch paths must be absolute\n</REASON>\n</FAILED>"
    );
}
