use super::{service, uuid_from};
use std::fs;
fn uuids_from(output: &str) -> Vec<String> {
    output
        .split("<UUID>\n")
        .skip(1)
        .map(|part| part.split_once("\n</UUID>").unwrap().0.to_owned())
        .collect()
}
fn two_uuids(output: &str) -> [String; 2] {
    let mut uuids = uuids_from(output).into_iter();
    let Some(first) = uuids.next() else {
        panic!("expected first operation UUID");
    };
    let Some(second) = uuids.next() else {
        panic!("expected second operation UUID");
    };
    assert!(uuids.next().is_none());
    [first, second]
}
#[test]
fn already_applied_update_can_be_undone_to_patch_before_contents() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.txt");
    fs::write(&target, "new\n").unwrap();
    let service = service(&directory);
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-old from patch\n+new\n*** End Patch",
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "new\n");
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "old from patch\n");
}
#[test]
fn already_applied_add_can_be_undone_to_a_missing_file() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.txt");
    fs::write(&target, "added\n").unwrap();
    let service = service(&directory);
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+added\n*** End Patch",
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "added\n");
    let undone = service.undo(&[uuid_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!target.exists());
    assert!(undone.render().contains("<DELETE>"));
}
#[test]
fn multiple_hunks_on_one_file_keep_independent_undo_history() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.txt");
    fs::write(&target, "one\n").unwrap();
    let service = service(&directory);
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-one\n+two\n*** Update File: {}\n@@\n-two\n+three\n*** End Patch",
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "three\n");
    let [first_uuid, second_uuid] = two_uuids(&applied.render());
    let undone_second = service.undo(core::slice::from_ref(&second_uuid));
    assert!(undone_second.succeeded(), "{}", undone_second.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "two\n");
    let undone_first = service.undo(core::slice::from_ref(&first_uuid));
    assert!(undone_first.succeeded(), "{}", undone_first.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "one\n");
}
#[test]
fn failed_hunk_does_not_discard_other_hunks() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.txt");
    let missing = directory.path().join("missing.txt");
    let second = directory.path().join("second.txt");
    let service = service(&directory);
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+first\n*** Update File: {}\n@@\n-old\n+new\n*** Add File: {}\n+second\n*** End Patch",
        first.display(),
        missing.display(),
        second.display()
    );
    let applied = service.apply(&patch);
    assert!(!applied.succeeded());
    assert_eq!(fs::read_to_string(&first).unwrap(), "first\n");
    assert_eq!(fs::read_to_string(&second).unwrap(), "second\n");
    let [first_uuid, second_uuid] = two_uuids(&applied.render());
    let undone = service.undo(&[first_uuid, second_uuid]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!first.exists());
    assert!(!second.exists());
}
#[test]
fn final_commit_failure_rolls_back_hunks_in_reverse_order() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.txt");
    fs::write(&target, "one\n").unwrap();
    let service = service(&directory);
    {
        let connection = service.history.lock_connection().unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER force_deferred_failure AFTER INSERT ON operations BEGIN
                 INSERT INTO operation_files VALUES(randomblob(16), -1, 0, X'', NULL, NULL);
                 END;
                 PRAGMA defer_foreign_keys = ON;",
            )
            .unwrap();
    }
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-one\n+two\n*** Update File: {}\n@@\n-two\n+three\n*** End Patch",
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(!applied.succeeded());
    assert_eq!(uuids_from(&applied.render()), Vec::<String>::new());
    assert_eq!(fs::read_to_string(target).unwrap(), "one\n");
    let connection = service.history.lock_connection().unwrap();
    let operation_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .unwrap();
    drop(connection);
    assert_eq!(operation_count, 0);
}
