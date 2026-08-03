use super::HistoryStore;
use crate::operation::model::{FileState, Mutation, OperationKind};
use rusqlite::TransactionBehavior;
use std::path::PathBuf;
fn present(contents: &str) -> FileState {
    FileState::present(contents.to_owned())
}
fn store(directory: &tempfile::TempDir) -> HistoryStore {
    HistoryStore::open(&directory.path().join("history.sqlite3")).unwrap()
}
#[test]
fn states_round_trip_and_shared_contents_are_stored_once() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let shared = "same\r\nUnicode: \u{4f60}\u{597d}";
    let mutation = Mutation::moved(
        PathBuf::from("source.txt"),
        PathBuf::from("destination.txt"),
        present(shared),
        present("displaced\n"),
        present(shared),
    );
    let mut connection = store.lock_connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let id = store.insert(&transaction, &mutation, None).unwrap();
    let stored = HistoryStore::load(&transaction, id).unwrap();
    assert_eq!(stored.changes, mutation.changes);
    let blob_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM history_blobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(blob_count, 2);
    transaction.commit().unwrap();
    drop(connection);
}
#[test]
fn chooses_raw_and_lz4_encodings_by_stored_size() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let compressible = "repeated source line\n".repeat(512);
    let mutation = Mutation::single(
        OperationKind::Edit,
        PathBuf::from("target.txt"),
        present("x"),
        present(&compressible),
    );
    let mut connection = store.lock_connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    store.insert(&transaction, &mutation, None).unwrap();
    let mut statement = transaction
        .prepare("SELECT codec FROM history_blobs ORDER BY raw_len")
        .unwrap();
    let codecs = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(codecs, [0, 1]);
    drop(statement);
    transaction.commit().unwrap();
    drop(connection);
}
#[test]
fn corrupted_raw_content_is_rejected_by_digest_validation() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mutation = Mutation::single(
        OperationKind::Add,
        PathBuf::from("target.txt"),
        FileState::Missing,
        present("x"),
    );
    let mut connection = store.lock_connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let id = store.insert(&transaction, &mutation, None).unwrap();
    transaction
        .execute("UPDATE history_blobs SET payload = ?1", [b"y".as_slice()])
        .unwrap();
    let error = HistoryStore::load(&transaction, id).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("operation history content digest mismatch")
    );
    transaction.rollback().unwrap();
    drop(connection);
}
#[test]
fn corrupted_lz4_content_is_rejected_during_decompression() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mutation = Mutation::single(
        OperationKind::Add,
        PathBuf::from("target.txt"),
        FileState::Missing,
        present(&"compress me\n".repeat(512)),
    );
    let mut connection = store.lock_connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let id = store.insert(&transaction, &mutation, None).unwrap();
    transaction
        .execute("UPDATE history_blobs SET payload = X'FF'", [])
        .unwrap();
    let error = HistoryStore::load(&transaction, id).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("failed to decompress operation history content")
    );
    transaction.rollback().unwrap();
    drop(connection);
}
#[test]
fn pruning_removes_content_with_its_last_operation_reference() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    for index in 0_u16..=1_000 {
        let mutation = Mutation::single(
            OperationKind::Add,
            PathBuf::from(format!("{index}.txt")),
            FileState::Missing,
            present(&format!("contents {index}")),
        );
        store.insert(&transaction, &mutation, None).unwrap();
    }
    store.prune(&transaction).unwrap();
    let operation_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
        .unwrap();
    let blob_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM history_blobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(operation_count, 1_000);
    assert_eq!(blob_count, 1_000);
    transaction.commit().unwrap();
    drop(connection);
}
