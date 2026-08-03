use super::{service, uuid_from};
use std::fs;
#[test]
fn requires_exactly_one_match() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "same\nsame\n").unwrap();
    let path = target.display().to_string();
    let output = service.replace(&path, "same", "changed");
    assert!(!output.succeeded());
    assert!(
        output
            .render()
            .contains("old_string must match exactly once; found 2 matches")
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "same\nsame\n");
}
#[test]
fn can_be_undone() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old value\n").unwrap();
    let path = target.display().to_string();
    let output = service.replace(&path, "old", "new");
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "new value\n");
    let undone = service.undo(&[uuid_from(&output.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "old value\n");
}
