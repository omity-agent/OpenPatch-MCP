use super::{apply, assert_contents, id_from, ids_from, service, target, undo};
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
#[test]
fn idempotent_and_repeated_updates_keep_one_undoable_operation() {
    for (original, expected, restored, body) in [
        (
            "new\n",
            "new\n",
            Some("old from patch\n"),
            "*** Update File: {path}\n@@\n-old from patch\n+new",
        ),
        ("added\n", "added\n", None, "*** Add File: {path}\n+added"),
        (
            "one\n",
            "three\n",
            Some("one\n"),
            "*** Update File: {path}\n@@\n-one\n+two\n*** Update File: {path}\n@@\n-two\n+three",
        ),
    ] {
        let (_directory, service, target) = target(original);
        let applied = apply(
            &service,
            &body.replace("{path}", &target.display().to_string()),
        );
        assert_eq!(ids_from(&applied).len(), 1);
        assert_contents(&target, Some(expected));
        let undone = undo(&service, &id_from(&applied));
        assert_contents(&target, restored);
        if restored.is_none() {
            assert!(undone.render().contains("<DELETE>"));
        }
    }
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
    let [first_id, second_id]: [String; 2] = ids_from(&applied.render()).try_into().unwrap();
    let undone = service.undo(&[first_id, second_id]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!first.exists());
    assert!(!second.exists());
}
