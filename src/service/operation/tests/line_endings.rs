use super::{apply, service, uuid_from};
use std::fs;
#[test]
fn new_file_uses_lf_independently_of_patch_line_endings() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    let patch = format!(
        "*** Begin Patch\r\n*** Add File: {}\n+first\r\n+second\n*** End Patch",
        target.display()
    );
    let output = service.apply(&patch);
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "first\nsecond\n");
}
#[test]
fn update_preserves_crlf_line_endings() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\r\nkeep\r\n").unwrap();
    apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    assert_eq!(fs::read_to_string(target).unwrap(), "new\r\nkeep\r\n");
}
#[test]
fn update_normalizes_mixed_line_endings_to_lf() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\r\nkeep\nlast\r\n").unwrap();
    apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    assert_eq!(fs::read_to_string(target).unwrap(), "new\nkeep\nlast\n");
}
#[test]
fn already_applied_update_still_normalizes_mixed_line_endings() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "new\r\nkeep\n").unwrap();
    apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    assert_eq!(fs::read_to_string(target).unwrap(), "new\nkeep\n");
}
#[test]
fn replacement_preserves_crlf_line_endings() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\r\nkeep\r\n").unwrap();
    let output = service.replace(&target.display().to_string(), "old\nkeep", "new\nchanged");
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "new\r\nchanged\r\n");
}
#[test]
fn undo_preserves_crlf_with_an_unrelated_edit() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\r\nkeep\r\n").unwrap();
    let output = apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    fs::write(&target, "new\r\nchanged\r\n").unwrap();
    let undone = service.undo(&[uuid_from(&output)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "old\r\nchanged\r\n");
}
#[test]
fn undo_normalizes_mixed_line_endings_to_lf() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\nkeep\n").unwrap();
    let output = apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    fs::write(&target, "new\r\nchanged\n").unwrap();
    let undone = service.undo(&[uuid_from(&output)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "old\nchanged\n");
}
