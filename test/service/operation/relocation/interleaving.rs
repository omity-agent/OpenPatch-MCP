use super::{assert_operations, service};
use std::fs;
#[test]
fn edit_then_move_keeps_separate_ids_and_requires_reverse_undo_order() {
    for (combined, initial) in [(false, "old\n"), (true, "old\n"), (true, "updated\n")] {
        let directory = tempfile::tempdir().unwrap();
        let service = service(&directory);
        let source = directory.path().join("source.txt");
        let destination = directory.path().join("destination.txt");
        fs::write(&source, initial).unwrap();
        fs::write(&destination, "displaced\n").unwrap();
        let body = if combined {
            format!(
                "*** Update File: {}\n*** Move to: {}\n@@\n-old\n+updated",
                source.display(),
                destination.display()
            )
        } else {
            format!(
                "*** Update File: {}\n@@\n-old\n+updated\n*** Update File: {}\n*** Move to: {}",
                source.display(),
                source.display(),
                destination.display()
            )
        };
        let applied = service.apply(&format!("*** Begin Patch\n{body}\n*** End Patch"));
        let [edit_id, move_id]: [String; 2] =
            assert_operations(&applied, &[("EDIT", &source), ("MOVE", &destination)])
                .try_into()
                .unwrap();
        assert_ne!(edit_id, move_id);
        let structured = applied.structured();
        assert_eq!(
            structured["successes"][0]["before"]["characterCount"],
            4_u64
        );
        assert_eq!(structured["successes"][0]["after"]["characterCount"], 8_u64);
        assert!(!source.exists());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "updated\n");
        drop(service);
        let reopened = super::service(&directory);
        let premature = reopened.undo(core::slice::from_ref(&edit_id));
        assert!(!premature.succeeded());
        assert_eq!(premature.structured()["successes"], sonic_rs::json!([]));
        assert_eq!(premature.structured()["failures"][0]["undoId"], edit_id);
        assert!(
            premature
                .render()
                .contains("edited file no longer exists at its recorded path")
        );
        assert!(!source.exists());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "updated\n");
        let moved_back = reopened.undo(core::slice::from_ref(&move_id));
        assert_operations(&moved_back, &[("MOVE", &source)]);
        assert_eq!(moved_back.structured()["successes"][0]["undoOf"], move_id);
        assert_eq!(fs::read_to_string(&source).unwrap(), "updated\n");
        assert_eq!(fs::read_to_string(&destination).unwrap(), "displaced\n");
        let restored = reopened.undo(core::slice::from_ref(&edit_id));
        assert_operations(&restored, &[("EDIT", &source)]);
        assert_eq!(restored.structured()["successes"][0]["undoOf"], edit_id);
        assert_eq!(fs::read_to_string(source).unwrap(), "old\n");
    }
}
#[test]
fn move_then_edit_keeps_the_edit_at_the_destination() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let source = directory.path().join("source.txt");
    let destination = directory.path().join("destination.txt");
    fs::write(&source, "old\n").unwrap();
    let applied = service.apply(&format!(
        "*** Begin Patch
*** Update File: {}
*** Move to: {}
*** Update File: {}
@@
-old
+updated
*** End Patch",
        source.display(),
        destination.display(),
        destination.display()
    ));
    let [move_id, edit_id]: [String; 2] =
        assert_operations(&applied, &[("MOVE", &destination), ("EDIT", &destination)])
            .try_into()
            .unwrap();
    assert_ne!(move_id, edit_id);
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "updated\n");
    let edited_back = service.undo(&[edit_id]);
    assert_operations(&edited_back, &[("EDIT", &destination)]);
    assert_eq!(fs::read_to_string(&destination).unwrap(), "old\n");
    assert!(!source.exists());
    let moved_back = service.undo(&[move_id]);
    assert_operations(&moved_back, &[("MOVE", &source)]);
    assert_eq!(fs::read_to_string(source).unwrap(), "old\n");
    assert!(!destination.exists());
}
#[test]
fn an_edit_between_moves_blocks_move_chain_coalescing() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let source = directory.path().join("source.txt");
    let middle = directory.path().join("middle.txt");
    let destination = directory.path().join("destination.txt");
    fs::write(&source, "old\n").unwrap();
    let applied = service.apply(&format!(
        "*** Begin Patch
*** Update File: {}
*** Move to: {}
*** Update File: {}
*** Move to: {}
@@
-old
+updated
*** End Patch",
        source.display(),
        middle.display(),
        middle.display(),
        destination.display()
    ));
    let ids = assert_operations(
        &applied,
        &[("MOVE", &middle), ("EDIT", &middle), ("MOVE", &destination)],
    );
    assert!(!source.exists());
    assert!(!middle.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "updated\n");
    let undone = service.undo(&ids);
    assert_operations(
        &undone,
        &[("MOVE", &middle), ("EDIT", &middle), ("MOVE", &source)],
    );
    assert_eq!(fs::read_to_string(source).unwrap(), "old\n");
    assert!(!middle.exists());
    assert!(!destination.exists());
}
