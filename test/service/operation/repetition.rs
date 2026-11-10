use super::{apply, assert_contents, id_from, ids_from, target, undo};
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
