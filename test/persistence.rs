use super::{HistoryStore, rows::IntegerValue};
use crate::operation::model::{FileState, Mutation, OperationKind};
use diesel::{RunQueryDsl as _, sql_types::Binary};
use std::path::PathBuf;
#[path = "persistence/allocation.rs"]
mod allocation;
#[path = "persistence/integrity.rs"]
mod integrity;
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
    );
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            let id = store.insert(transaction, &mutation, None)?;
            let stored = HistoryStore::load(transaction, id)?;
            assert_eq!(stored.changes, mutation.changes);
            assert_eq!(count(transaction, "history_blobs")?, 2);
            anyhow::Ok(())
        })
        .unwrap();
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
    connection
        .immediate_transaction(|transaction| {
            store.insert(transaction, &mutation, None)?;
            let codecs =
                diesel::sql_query("SELECT codec AS value FROM history_blobs ORDER BY raw_len")
                    .load::<IntegerValue>(transaction)?
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>();
            assert_eq!(codecs, [0, 1]);
            anyhow::Ok(())
        })
        .unwrap();
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
    connection
        .immediate_transaction(|transaction| {
            let id = store.insert(transaction, &mutation, None)?;
            diesel::sql_query("UPDATE history_blobs SET payload = ?")
                .bind::<Binary, _>(b"y".as_slice())
                .execute(transaction)?;
            let error = HistoryStore::load(transaction, id).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("operation history content digest mismatch")
            );
            anyhow::Ok(())
        })
        .unwrap();
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
    connection
        .immediate_transaction(|transaction| {
            let id = store.insert(transaction, &mutation, None)?;
            diesel::sql_query("UPDATE history_blobs SET payload = X'FF'").execute(transaction)?;
            let error = HistoryStore::load(transaction, id).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("failed to decompress operation history content")
            );
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
#[test]
fn pruning_removes_content_with_its_last_operation_reference() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            for index in 0_u16..=1_000 {
                let mutation = Mutation::single(
                    OperationKind::Add,
                    PathBuf::from(format!("{index}.txt")),
                    FileState::Missing,
                    present(&format!("contents {index}")),
                );
                store.insert(transaction, &mutation, None)?;
            }
            store.prune(transaction)?;
            assert_eq!(count(transaction, "operations")?, 1_000);
            assert_eq!(count(transaction, "history_blobs")?, 1_000);
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
fn count(connection: &mut diesel::SqliteConnection, table: &str) -> anyhow::Result<i64> {
    let query = format!("SELECT COUNT(*) AS value FROM {table}");
    Ok(diesel::sql_query(query)
        .get_result::<IntegerValue>(connection)?
        .value)
}
