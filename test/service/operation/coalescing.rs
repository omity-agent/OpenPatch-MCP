use super::{apply, assert_contents, id_from, service, target, undo};
use std::fs;
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
        let applied = apply(
            &service,
            &body.replace("{path}", &target.display().to_string()),
        );
        assert_single_kind(&applied, kind);
        assert_contents(&target, expected);
        undo(&service, &id_from(&applied));
        assert_eq!(fs::read_to_string(target).unwrap(), original);
    }
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
    let applied = apply(
        &service,
        &format!(
            "*** Update File: {}\n*** Move to: {}\n*** Update File: {}\n*** Move to: {}",
            source.display(),
            middle.display(),
            middle.display(),
            destination.display()
        ),
    );
    assert_single_kind(&applied, "MOVE");
    assert!(!source.exists());
    assert!(!middle.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "source\n");
    let undone = undo(&service, &id_from(&applied));
    assert_single_kind(&undone.render(), "MOVE");
    assert_eq!(fs::read_to_string(source).unwrap(), "source\n");
    assert!(!middle.exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "destination\n");
}
