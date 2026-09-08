use super::{HistoryStore, store};
use crate::operation::{
    model::{FileState, Mutation, OperationId, OperationKind},
    store::{for_undo, recording::reserve},
};
use diesel::{RunQueryDsl as _, connection::SimpleConnection as _, sql_types::BigInt};
use std::path::PathBuf;
#[test]
fn schema_accepts_only_in_range_integer_ids_and_references() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut connection = store.lock_connection().unwrap();
    for boundary in [0, OperationId::LIMIT - 1] {
        diesel::sql_query("INSERT INTO operations(id, kind, display_path) VALUES (?, 0, X'')")
            .bind::<BigInt, _>(boundary)
            .execute(&mut *connection)
            .unwrap();
    }
    for invalid in [
        "-1",
        "2821109907456",
        "1.5",
        "'oppc-00000000'",
        "X'00000000'",
    ] {
        let query =
            format!("INSERT INTO operations(id, kind, display_path) VALUES ({invalid}, 0, X'')");
        assert!(connection.batch_execute(&query).is_err(), "{invalid}");
        for field in ["undo_of", "undone_by"] {
            let update = format!("UPDATE operations SET {field} = {invalid} WHERE id = 0");
            assert!(
                connection.batch_execute(&update).is_err(),
                "{field}: {invalid}"
            );
        }
    }
    drop(connection);
}
#[test]
fn undo_order_uses_record_sequence_instead_of_random_id_values() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mutation = Mutation::single(
        OperationKind::Add,
        PathBuf::from("ordered.txt"),
        FileState::Missing,
        FileState::present(String::from("contents")),
    );
    let earlier = OperationId::parse("oppc-zzzzzzzz").unwrap();
    let later = OperationId::parse("oppc-00000000").unwrap();
    let unknown = OperationId::parse("oppc-12345678").unwrap();
    let mut connection = store.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            reserve(transaction, &mutation, None, || earlier)?;
            reserve(transaction, &mutation, None, || later)?;
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
    assert_eq!(
        for_undo(&store, &[earlier, unknown, later]).unwrap(),
        [2, 0, 1]
    );
    assert_eq!(for_undo(&store, &[later, earlier]).unwrap(), [0, 1]);
}
#[test]
fn committed_ids_can_be_loaded_after_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("reopened.sqlite3");
    let writer = HistoryStore::open(&database).unwrap();
    let mutation = Mutation::single(
        OperationKind::Add,
        PathBuf::from("persistent.txt"),
        FileState::Missing,
        FileState::present(String::from("saved")),
    );
    let mut connection = writer.lock_connection().unwrap();
    let id = connection
        .immediate_transaction(|transaction| writer.insert(transaction, &mutation, None))
        .unwrap();
    drop(connection);
    drop(writer);
    let reader = HistoryStore::open(&database).unwrap();
    let mut reopened = reader.lock_connection().unwrap();
    let stored =
        HistoryStore::load(&mut reopened, OperationId::parse(&id.to_string()).unwrap()).unwrap();
    assert_eq!(stored.changes, mutation.changes);
    drop(reopened);
}
