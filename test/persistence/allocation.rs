use super::{count, present, store};
use crate::operation::{
    model::{FileState, Mutation, OperationId, OperationKind},
    store::recording::reserve,
};
use diesel::{RunQueryDsl as _, sql_types::BigInt};
use std::path::PathBuf;
fn mutation() -> Mutation {
    Mutation::single(
        OperationKind::Add,
        PathBuf::from("allocated.txt"),
        FileState::Missing,
        present("contents"),
    )
}
#[test]
fn collisions_retry_without_overwriting_the_existing_operation() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            let existing = store.insert(transaction, &mutation(), None)?;
            let available = OperationId::from_stored(i64::from(existing.value() == 0))?;
            let mut attempts = 0_u8;
            let allocated = reserve(transaction, &mutation(), None, || {
                attempts += 1;
                if attempts < 3 { existing } else { available }
            })?;
            assert_eq!(allocated, available);
            assert_eq!(attempts, 3);
            assert_eq!(count(transaction, "operations")?, 2);
            let stored = super::HistoryStore::load(transaction, existing)?;
            assert_eq!(stored.changes, mutation().changes);
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
#[test]
fn repeated_collisions_stop_after_the_attempt_limit() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            let existing = store.insert(transaction, &mutation(), None)?;
            let mut attempts = 0_u8;
            let error = reserve(transaction, &mutation(), None, || {
                attempts += 1;
                existing
            })
            .unwrap_err();
            assert_eq!(attempts, 16);
            assert!(error.to_string().contains("after 16 attempts"));
            assert_eq!(count(transaction, "operations")?, 1);
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
#[test]
fn unrelated_unique_constraint_errors_are_not_retried() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            let target = OperationId::from_stored(0)?;
            let first = OperationId::from_stored(1)?;
            let second = OperationId::from_stored(2)?;
            reserve(transaction, &mutation(), Some(target), || first)?;
            let mut attempts = 0_u8;
            let error = reserve(transaction, &mutation(), Some(target), || {
                attempts += 1;
                second
            })
            .unwrap_err();
            assert_eq!(attempts, 1);
            assert!(error.is::<diesel::result::Error>());
            assert_eq!(count(transaction, "operations")?, 1);
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
#[test]
fn allocation_and_contents_roll_back_together() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    let mut reserved = None;
    let result: anyhow::Result<()> = connection.immediate_transaction(|transaction| {
        reserved = Some(store.insert(transaction, &mutation(), None)?);
        anyhow::bail!("abort operation");
    });
    assert!(result.is_err());
    assert!(reserved.is_some());
    assert_eq!(count(&mut connection, "operations").unwrap(), 0);
    assert_eq!(count(&mut connection, "operation_files").unwrap(), 0);
    assert_eq!(count(&mut connection, "history_blobs").unwrap(), 0);
    let id = reserved.unwrap();
    let rows = diesel::sql_query("SELECT COUNT(*) AS value FROM operations WHERE id = ?")
        .bind::<BigInt, _>(id.value())
        .get_result::<super::IntegerValue>(&mut *connection)
        .unwrap();
    assert_eq!(rows.value, 0);
    drop(connection);
}
