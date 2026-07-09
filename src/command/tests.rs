use super::{PatchExecution, PatchOutput, PatchRunner};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
fn success_requires_zero_status() {
    assert!(
        PatchOutput {
            status: Some(0_i32),
            stdout: String::new(),
            stderr: String::new()
        }
        .succeeded()
    );
}
#[test]
fn multiline_patch_is_applied_without_executable() {
    let directory = unique_temp_directory().unwrap();
    let target_path = directory.join("target.txt");
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
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn failed_update_does_not_stop_following_files() {
    let directory = unique_temp_directory().unwrap();
    let first_path = directory.join("a.txt");
    let second_path = directory.join("b.txt");
    let third_path = directory.join("c.txt");
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
        "-missing",
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
    assert!(
        output
            .stdout
            .contains(&format!("M {}", first_path.display()))
    );
    assert!(
        output
            .stdout
            .contains(&format!("M {}", third_path.display()))
    );
    assert!(output.stderr.contains("Failed to find expected lines"));
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn delete_missing_file_reports_delete_context() {
    let directory = unique_temp_directory().unwrap();
    let patch = [
        "*** Begin Patch",
        &format!(
            "*** Delete File: {}",
            directory.join("missing.txt").display()
        ),
        "*** End Patch",
        "",
    ]
    .join("\n");
    let output = PatchRunner::apply(PatchExecution { patch: &patch });
    assert!(!output.succeeded());
    assert!(output.stderr.contains("Failed to delete file"));
    assert!(!output.stderr.contains("Failed to inspect file"));
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn delete_directory_reports_reference_style_context() {
    let directory = unique_temp_directory().unwrap();
    let target_path = directory.join("target");
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
    assert!(output.stderr.contains("Failed to delete file"));
    assert!(output.stderr.contains("path is a directory"));
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn absolute_patch_path_is_applied() {
    let directory = unique_temp_directory().unwrap();
    let target_path = directory.join("target.txt");
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
    assert!(
        output
            .stdout
            .contains(&format!("M {}", target_path.display()))
    );
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn relative_patch_path_is_rejected() {
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
    assert!(output.stderr.contains("patch paths must be absolute"));
}
fn unique_temp_directory() -> anyhow::Result<PathBuf> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory =
        std::env::temp_dir().join(format!("apply-patch-mcp-{}-{suffix}", std::process::id()));
    fs::create_dir_all(&directory)?;
    Ok(directory)
}
