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
    assert_eq!(fs::read_to_string(&target).unwrap(), "old\nkeep changed\n");
    undo(&service, &id_from(&undone.render()));
    assert_eq!(fs::read_to_string(target).unwrap(), "new\nkeep changed\n");
}
#[test]
fn undo_add_restores_overwritten_file() {
    let (_directory, service, target) = target("original\n");
    let applied = apply(
        &service,
        &format!("*** Add File: {}\n+replacement", target.display()),
    );
    let undone = undo(&service, &id_from(&applied));
    assert_eq!(fs::read_to_string(target).unwrap(), "original\n");
    assert!(undone.render().contains("<EDIT>"));
}
#[test]
fn undo_move_restores_source_and_overwritten_destination() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let source = directory.path().join("source.txt");
    let destination = directory.path().join("destination.txt");
    fs::write(&source, "source\n").unwrap();
    fs::write(&destination, "displaced\n").unwrap();
    let applied = apply(
        &service,
        &format!(
            "*** Update File: {}\n*** Move to: {}",
            source.display(),
            destination.display()
        ),
    );
    assert!(applied.contains("<MOVE>"));
    fs::write(&destination, "source\nextra\n").unwrap();
    let undone = undo(&service, &id_from(&applied));
    assert!(undone.render().contains("<MOVE>"));
    assert_eq!(fs::read_to_string(source).unwrap(), "source\nextra\n");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "displaced\n");
    let redone = undo(&service, &id_from(&undone.render()));
    assert!(redone.render().contains("<MOVE>"));
    assert!(!directory.path().join("source.txt").exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "source\nextra\n");
}
