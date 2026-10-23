use super::{id_from, ids_from, service};
use core::fmt::Write as _;
use std::fs;
#[test]
fn large_batches_keep_first_operation_order_and_undo_every_file() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let paths = (0_u16..128)
        .map(|index| directory.path().join(format!("{index}.txt")))
        .collect::<Vec<_>>();
    let mut patch = String::from("*** Begin Patch\n");
    for path in &paths {
        writeln!(patch, "*** Add File: {}\n+first", path.display()).unwrap();
    }
    for path in paths.iter().rev() {
        writeln!(
            patch,
            "*** Update File: {}\n@@\n-first\n+final",
            path.display()
        )
        .unwrap();
    }
    patch.push_str("*** End Patch");
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    let structured = applied.structured();
    for (index, path) in paths.iter().enumerate() {
        assert_eq!(
            structured["successes"][index]["path"],
            path.display().to_string()
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "final\n");
    }
    let ids = ids_from(&applied.render());
    assert_eq!(ids.len(), paths.len());
    let undone = service.undo(&ids);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(paths.iter().all(|path| !path.exists()));
}
fn two_ids(output: &str) -> [String; 2] {
    let mut ids = ids_from(output).into_iter();
    let Some(first) = ids.next() else {
        panic!("expected first operation ID");
    };
    let Some(second) = ids.next() else {
        panic!("expected second operation ID");
    };
    assert!(ids.next().is_none());
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
    let undone = service.undo(&[id_from(&applied.render())]);
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
    let undone = service.undo(&[id_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!target.exists());
    assert!(undone.render().contains("<DELETE>"));
}
#[test]
fn multiple_hunks_on_one_file_share_one_undo_history_entry() {
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
    assert_eq!(ids_from(&applied.render()).len(), 1);
    let undone = service.undo(&[id_from(&applied.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
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
    let [first_id, second_id] = two_ids(&applied.render());
    let undone = service.undo(&[first_id, second_id]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!first.exists());
    assert!(!second.exists());
}
