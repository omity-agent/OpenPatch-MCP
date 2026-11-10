use super::{apply, assert_contents, id_from, target, undo};
fn assert_single_kind(output: &str, kind: &str) {
    assert_eq!(output.matches("<ID>").count(), 1, "{output}");
    assert!(output.contains(&format!("<{kind}>")), "{output}");
    for other in ["ADD", "EDIT", "DELETE", "MOVE"] {
        if other != kind {
            assert!(!output.contains(&format!("<{other}>")), "{output}");
        }
    }
}
#[test]
fn edits_and_deletes_coalesce_and_remain_undoable() {
    for (original, kind, expected, body) in [
        (
            "original\n",
            "EDIT",
            Some("final\n"),
            "*** Delete File: {path}\n*** Add File: {path}\n+replacement\n*** Update File: {path}\n@@\n-replacement\n+final",
        ),
        (
            "one\n",
            "DELETE",
            None,
            "*** Update File: {path}\n@@\n-one\n+two\n*** Update File: {path}\n@@\n-two\n+three\n*** Delete File: {path}",
        ),
        (
            "added\n",
            "DELETE",
            None,
            "*** Add File: {path}\n+added\n*** Delete File: {path}",
        ),
    ] {
        let (_directory, service, target) = target(original);
        let patch_body = body.replace("{path}", &target.display().to_string());
        let applied = apply(&service, &patch_body);
        assert_single_kind(&applied, kind);
        assert_contents(&target, expected);
        undo(&service, &id_from(&applied));
        assert_contents(&target, Some(original));
    }
}
