use super::{OperationService, model::OperationId};
use std::fs;
#[path = "operation/applying.rs"]
mod applying;
#[path = "operation/coalescing.rs"]
mod coalescing;
#[path = "operation/history.rs"]
mod history;
#[path = "operation/line_endings.rs"]
mod line_endings;
#[path = "operation/replacement.rs"]
mod replacement;
#[path = "operation/validation.rs"]
mod validation;
fn id_from(output: &str) -> String {
    let (_, after_start) = output.split_once("<ID>\n").unwrap();
    let (id, _) = after_start.split_once("\n</ID>").unwrap();
    assert_eq!(OperationId::parse(id).unwrap().to_string(), id);
    id.to_owned()
}
fn apply(service: &OperationService, lines: &[String]) -> String {
    let output = service.apply(&lines.join("\n"));
    assert!(output.succeeded(), "{}", output.render());
    output.render()
}
fn service(directory: &tempfile::TempDir) -> OperationService {
    OperationService::open(&directory.path().join("history.sqlite3")).unwrap()
}
#[test]
fn edit_undo_preserves_unrelated_changes_and_can_be_undone() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\nkeep\n").unwrap();
    let applied = apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", target.display()),
            String::from("@@"),
            String::from("-old"),
            String::from("+new"),
            String::from("*** End Patch"),
        ],
    );
    fs::write(&target, "new\nkeep changed\n").unwrap();
    let first_id = id_from(&applied);
    let undone = service.undo(core::slice::from_ref(&first_id));
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "old\nkeep changed\n");
    let redone = service.undo(&[id_from(&undone.render())]);
    assert!(redone.succeeded(), "{}", redone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "new\nkeep changed\n");
}
#[test]
fn undo_add_restores_overwritten_file() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "original\n").unwrap();
    let applied = apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Add File: {}", target.display()),
            String::from("+replacement"),
            String::from("*** End Patch"),
        ],
    );
    let undone = service.undo(&[id_from(&applied)]);
    assert!(undone.succeeded(), "{}", undone.render());
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
        &[
            String::from("*** Begin Patch"),
            format!("*** Update File: {}", source.display()),
            format!("*** Move to: {}", destination.display()),
            String::from("*** End Patch"),
        ],
    );
    fs::write(&destination, "source\nextra\n").unwrap();
    let undone = service.undo(&[id_from(&applied)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(source).unwrap(), "source\nextra\n");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "displaced\n");
    let redone = service.undo(&[id_from(&undone.render())]);
    assert!(redone.succeeded(), "{}", redone.render());
    assert!(!directory.path().join("source.txt").exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "source\nextra\n");
}
#[test]
fn separate_service_instances_share_the_wal_history() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("history.sqlite3");
    let writer = OperationService::open(&database).unwrap();
    let reader = OperationService::open(&database).unwrap();
    let target = directory.path().join("shared.txt");
    let applied = apply(
        &writer,
        &[
            String::from("*** Begin Patch"),
            format!("*** Add File: {}", target.display()),
            String::from("+shared"),
            String::from("*** End Patch"),
        ],
    );
    let undone = reader.undo(&[id_from(&applied)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!target.exists());
}
#[test]
fn batch_undo_keeps_successes_beside_invalid_id_failure() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let first = directory.path().join("first.txt");
    let second = directory.path().join("second.txt");
    let applied = apply(
        &service,
        &[
            String::from("*** Begin Patch"),
            format!("*** Add File: {}", first.display()),
            String::from("+first"),
            format!("*** Add File: {}", second.display()),
            String::from("+second"),
            String::from("*** End Patch"),
        ],
    );
    let mut id_iterator = applied
        .split("<ID>\n")
        .skip(1)
        .map(|part| part.split_once("\n</ID>").unwrap().0.to_owned());
    let Some(first_id) = id_iterator.next() else {
        panic!("expected first operation ID");
    };
    let Some(second_id) = id_iterator.next() else {
        panic!("expected second operation ID");
    };
    assert!(id_iterator.next().is_none());
    let result = service.undo(&[first_id, String::from("not-an-id"), second_id]);
    assert!(!result.succeeded());
    assert!(result.render().contains("<SUCCEEDED>"));
    assert!(result.render().contains("<FAILED>"));
    assert!(!first.exists());
    assert!(!second.exists());
}
