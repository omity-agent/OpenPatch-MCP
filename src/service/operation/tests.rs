use super::{
    OperationService,
    model::{FileState, Mutation, OperationId, OperationKind},
};
use rusqlite::TransactionBehavior;
use std::fs;
fn uuid_from(output: &str) -> String {
    let (_, after_start) = output.split_once("<UUID>\n").unwrap();
    let (uuid, _) = after_start.split_once("\n</UUID>").unwrap();
    uuid.to_owned()
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
fn operation_ids_require_hyphenated_format() {
    const UUID: &str = "019d0000-0000-7000-8000-000000000103";
    let operation_id = OperationId::parse(UUID).unwrap();
    assert_eq!(operation_id.to_string(), UUID);
    for invalid_uuid in [
        "019d0000000070008000000000000103",
        "019d0000_0000-7000-8000-000000000103",
    ] {
        let error = OperationId::parse(invalid_uuid).unwrap_err();
        assert_eq!(error.to_string(), "invalid UUID");
    }
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
    let first_uuid = uuid_from(&applied);
    let undone = service.undo(core::slice::from_ref(&first_uuid));
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "old\nkeep changed\n");
    let redone = service.undo(&[uuid_from(&undone.render())]);
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
    let undone = service.undo(&[uuid_from(&applied)]);
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
    let undone = service.undo(&[uuid_from(&applied)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(source).unwrap(), "source\nextra\n");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "displaced\n");
    let redone = service.undo(&[uuid_from(&undone.render())]);
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
    let undone = reader.undo(&[uuid_from(&applied)]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert!(!target.exists());
}
#[test]
fn history_retention_uses_only_the_latest_thousand_records() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let mut connection = service.history.connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    for index in 0_u16..1_001 {
        let mutation = Mutation::single(
            OperationKind::Edit,
            directory.path().join(format!("{index}.txt")),
            FileState::Missing,
            FileState::Missing,
        );
        service
            .history
            .insert(&transaction, &mutation, None)
            .unwrap();
        service.history.prune(&transaction).unwrap();
    }
    let count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1000);
    transaction.commit().unwrap();
}
#[test]
fn batch_undo_keeps_successes_beside_invalid_uuid_failure() {
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
    let mut uuid_iterator = applied
        .split("<UUID>\n")
        .skip(1)
        .map(|part| part.split_once("\n</UUID>").unwrap().0.to_owned());
    let Some(first_uuid) = uuid_iterator.next() else {
        panic!("expected first operation UUID");
    };
    let Some(second_uuid) = uuid_iterator.next() else {
        panic!("expected second operation UUID");
    };
    assert!(uuid_iterator.next().is_none());
    let result = service.undo(&[first_uuid, String::from("not-a-uuid"), second_uuid]);
    assert!(!result.succeeded());
    assert!(result.render().contains("<SUCCEEDED>"));
    assert!(result.render().contains("<FAILED>"));
    assert!(!first.exists());
    assert!(!second.exists());
}
