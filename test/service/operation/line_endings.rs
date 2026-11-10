use super::{assert_contents, id_from, target, undo};
use std::fs;
#[test]
fn updates_and_undo_preserve_newlines_and_normalize_mixed_newlines() {
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
        ("old\n\n", "new\n\n"),
        ("old\rkeep\r", "new\rkeep\r"),
        ("old\nkeep\n\n", "new\nkeep\n\n"),
        ("old\n\n\n", "new\n\n\n"),
    ] {
        fs::write(&target, original).unwrap();
        let output = service.apply(&patch);
        assert!(output.succeeded(), "{}", output.render());
        assert_contents(&target, Some(expected));
    }
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
        assert_contents(&target, Some(expected));
    }
}
#[test]
fn replacements_preserve_newlines() {
    let (_directory, service, target) = target("");
    for (original, old, new, expected) in [
        (
            "old\r\nkeep\r\n",
            "old\nkeep",
            "new\nchanged",
            "new\r\nchanged\r\n",
        ),
        ("old\nkeep", "old", "new", "new\nkeep"),
        ("keep\n\n", "", "append", "keep\n\nappend\n"),
        ("\n", "", "append", "\nappend\n"),
        ("", "", "append", "append"),
    ] {
        fs::write(&target, original).unwrap();
        let output = service.replace(&target.display().to_string(), old, new);
        assert!(output.succeeded(), "{}", output.render());
        assert_contents(&target, Some(expected));
        let repeated = service.replace(&target.display().to_string(), old, new);
        assert!(repeated.succeeded(), "{}", repeated.render());
        undo(&service, &id_from(&repeated.render()));
        assert_contents(&target, Some(original));
    }
}
