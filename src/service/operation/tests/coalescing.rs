use super::{service, uuid_from};
use std::fs;
fn assert_single_kind(output: &str, kind: &str) {
    assert_eq!(output.matches("<UUID>").count(), 1, "{output}");
    assert!(output.contains(&format!("<{kind}>")), "{output}");
    for other in ["ADD", "EDIT", "DELETE"] {
        if other != kind {
            assert!(!output.contains(&format!("<{other}>")), "{output}");
        }
    }
}
#[test]
fn delete_add_and_edit_are_coalesced_into_one_edit() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "original\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Delete File: {}
*** Add File: {}
+replacement
*** Update File: {}
@@
-replacement
+final
*** End Patch",
        target.display(),
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_single_kind(&applied.render(), "EDIT");
    assert_eq!(fs::read_to_string(&target).unwrap(), "final\n");
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "original\n");
}
#[test]
fn edits_followed_by_delete_are_coalesced_into_one_delete() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "one\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Update File: {}
@@
-one
+two
*** Update File: {}
@@
-two
+three
*** Delete File: {}
*** End Patch",
        target.display(),
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_single_kind(&applied.render(), "DELETE");
    assert!(!target.exists());
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "one\n");
}
#[test]
fn fully_cancelled_operations_return_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Update File: {}
@@
-old
+new
*** Update File: {}
@@
-new
+old
*** End Patch",
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(!applied.succeeded());
    assert!(
        applied
            .render()
            .contains("No effective file operations remained after merging.")
    );
    assert!(!applied.render().contains("<UUID>"));
    assert_eq!(fs::read_to_string(target).unwrap(), "old\n");
}
#[test]
fn already_applied_add_followed_by_delete_still_deletes_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "added\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Add File: {}
+added
*** Delete File: {}
*** End Patch",
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_single_kind(&applied.render(), "DELETE");
    assert!(!target.exists());
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "added\n");
}
#[test]
fn move_chain_is_coalesced_into_one_move() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let source = directory.path().join("source.txt");
    let middle = directory.path().join("middle.txt");
    let destination = directory.path().join("destination.txt");
    fs::write(&source, "source\n").unwrap();
    fs::write(&destination, "destination\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Update File: {}
*** Move to: {}
*** Update File: {}
*** Move to: {}
@@
-source
+final
*** End Patch",
        source.display(),
        middle.display(),
        middle.display(),
        destination.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_single_kind(&applied.render(), "EDIT");
    assert!(!source.exists());
    assert!(!middle.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "final\n");
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(source).unwrap(), "source\n");
    assert!(!middle.exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "destination\n");
}
