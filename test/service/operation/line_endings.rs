use super::{id_from, service, target, undo};
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
fn updates_preserve_original_newlines_and_normalize_mixed_newlines() {
    let (_directory, service, target) = target("");
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-old\n+new\n*** End Patch",
        target.display()
    );
    for (original, expected) in [
        ("old\r\nkeep\r\n", "new\r\nkeep\r\n"),
        ("old\r\nkeep", "new\r\nkeep"),
        ("old\r\nkeep\nlast\r\n", "new\nkeep\nlast\n"),
        ("new\r\nkeep\n", "new\nkeep\n"),
    ] {
        fs::write(&target, original).unwrap();
        let output = service.apply(&patch);
        assert!(output.succeeded(), "{}", output.render());
        assert_eq!(fs::read_to_string(&target).unwrap(), expected);
    }
}
#[test]
fn replacements_preserve_newlines() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    for (original, old, new, expected) in [
        (
            "old\r\nkeep\r\n",
            "old\nkeep",
            "new\nchanged",
            "new\r\nchanged\r\n",
        ),
        ("old\nkeep", "old", "new", "new\nkeep"),
    ] {
        fs::write(&target, original).unwrap();
        let output = service.replace(&target.display().to_string(), old, new);
        assert!(output.succeeded(), "{}", output.render());
        assert_eq!(fs::read_to_string(&target).unwrap(), expected);
    }
}
#[test]
fn undo_preserves_unrelated_edits_and_normalizes_mixed_newlines() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-old\n+new\n*** End Patch",
        target.display()
    );
    for (original, modified, expected) in [
        (
            "old\r\nkeep\r\n",
            "new\r\nchanged\r\n",
            "old\r\nchanged\r\n",
        ),
        ("old\nkeep\n", "new\r\nchanged\n", "old\nchanged\n"),
    ] {
        fs::write(&target, original).unwrap();
        let output = service.apply(&patch);
        assert!(output.succeeded(), "{}", output.render());
        fs::write(&target, modified).unwrap();
        undo(&service, &id_from(&output.render()));
        assert_eq!(fs::read_to_string(&target).unwrap(), expected);
    }
}
