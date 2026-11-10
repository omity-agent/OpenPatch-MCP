use super::{OperationService, model::OperationId, result::OperationOutput};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[path = "operation/coalescing.rs"]
mod coalescing;
#[path = "operation/line_endings.rs"]
mod line_endings;
#[path = "operation/relocation.rs"]
mod relocation;
#[path = "operation/repetition.rs"]
mod repetition;
#[path = "operation/replacement.rs"]
mod replacement;
#[path = "operation/rules.rs"]
mod rules;
#[path = "operation/validation.rs"]
mod validation;
fn id_from(output: &str) -> String {
    let (_, after_start) = output.split_once("<ID>\n").unwrap();
    let (id, _) = after_start.split_once("\n</ID>").unwrap();
    assert_eq!(OperationId::parse(id).unwrap().to_string(), id);
    id.to_owned()
}
fn ids_from(output: &str) -> Vec<String> {
    output
        .split("<ID>\n")
        .skip(1)
        .map(|part| part.split_once("\n</ID>").unwrap().0.to_owned())
        .collect()
}
fn apply(service: &OperationService, body: &str) -> String {
    let output = service.apply(&format!("*** Begin Patch\n{body}\n*** End Patch"));
    assert!(output.succeeded(), "{}", output.render());
    output.render()
}
fn undo(service: &OperationService, id: &str) -> OperationOutput {
    let output = service.undo(&[id.to_owned()]);
    assert!(output.succeeded(), "{}", output.render());
    output
}
fn service(directory: &tempfile::TempDir) -> OperationService {
    OperationService::open(&directory.path().join("history.lmdb")).unwrap()
}
fn target(contents: &str) -> (tempfile::TempDir, OperationService, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let path = directory.path().join("target.txt");
    fs::write(&path, contents).unwrap();
    (directory, service, path)
}
fn assert_contents(path: &Path, expected: Option<&str>) {
    match expected {
        Some(contents) => assert_eq!(fs::read_to_string(path).unwrap(), contents),
        None => assert!(!path.exists()),
    }
}
#[test]
fn edit_undo_preserves_unrelated_changes_and_can_be_undone() {
    let (_directory, service, target) = target("old\nkeep\n");
    let applied = apply(
        &service,
        &format!("*** Update File: {}\n@@\n-old\n+new", target.display()),
    );
    fs::write(&target, "new\nkeep changed\n").unwrap();
    let first_id = id_from(&applied);
    let undone = undo(&service, &first_id);
    assert_contents(&target, Some("old\nkeep changed\n"));
    undo(&service, &id_from(&undone.render()));
    assert_contents(&target, Some("new\nkeep changed\n"));
}
#[test]
fn undo_move_restores_source_and_overwritten_destination() {
    for names in [
        &["source.txt", "destination.txt"][..],
        &["source.txt", "middle.txt", "destination.txt"][..],
    ] {
        let directory = tempfile::tempdir().unwrap();
        let service = service(&directory);
        let root = directory.path();
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::write(&source, "source\n").unwrap();
        fs::write(&destination, "displaced\n").unwrap();
        let body = names
            .array_windows::<2>()
            .map(|&[from, to]| {
                format!(
                    "*** Update File: {}\n*** Move to: {}",
                    root.join(from).display(),
                    root.join(to).display()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let applied = apply(&service, &body);
        assert_eq!(ids_from(&applied).len(), 1);
        assert!(applied.contains("<MOVE>"));
        assert_contents(&source, None);
        assert!(!root.join("middle.txt").exists());
        fs::write(&destination, "source\nextra\n").unwrap();
        let undone = undo(&service, &id_from(&applied));
        assert!(undone.render().contains("<MOVE>"));
        assert_contents(&source, Some("source\nextra\n"));
        assert_contents(&destination, Some("displaced\n"));
        assert!(!root.join("middle.txt").exists());
        let redone = undo(&service, &id_from(&undone.render()));
        assert!(redone.render().contains("<MOVE>"));
        assert_contents(&source, None);
        assert_contents(&destination, Some("source\nextra\n"));
    }
}
