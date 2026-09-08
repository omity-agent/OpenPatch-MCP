use super::HistoryStore;
use crate::operation::model::{FileState, Mutation, OperationId, OperationKind};
use std::path::PathBuf;
#[path = "persistence/integrity.rs"]
mod integrity;
#[path = "persistence/interruption.rs"]
mod interruption;
#[path = "persistence/processes.rs"]
mod processes;
fn present(contents: &str) -> FileState {
    FileState::present(contents.to_owned())
}
fn store(directory: &tempfile::TempDir) -> alloc::sync::Arc<HistoryStore> {
    HistoryStore::open(&directory.path().join("history.lmdb")).unwrap()
}
fn record(store: &HistoryStore, mutation: &Mutation) -> OperationId {
    let ids = store
        .prepare(core::slice::from_ref(mutation), &[], None)
        .unwrap();
    assert_eq!(store.finish().unwrap(), ids);
    *ids.first().unwrap()
}
fn mutation() -> Mutation {
    Mutation::single(
        OperationKind::Add,
        PathBuf::from("allocated.txt"),
        FileState::Missing,
        present("contents"),
    )
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
    let id = record(&store, &mutation);
    assert_eq!(store.load(id).unwrap().changes, mutation.changes);
    let transaction = store.env.read_txn().unwrap();
    assert_eq!(store.contents.len(&transaction).unwrap(), 2);
    assert_eq!(store.references.len(&transaction).unwrap(), 2);
}
#[test]
fn pruning_releases_only_last_references_and_keeps_newest_thousand() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mutations = (0_u16..1001)
        .map(|index| {
            Mutation::single(
                OperationKind::Edit,
                PathBuf::from(format!("{index}.txt")),
                present("shared"),
                present(&format!("contents {index}")),
            )
        })
        .collect::<Vec<_>>();
    let ids = store.prepare(&mutations, &[], None).unwrap();
    store.finish().unwrap();
    store.load(*ids.first().unwrap()).unwrap_err();
    store.load(*ids.last().unwrap()).unwrap();
    let transaction = store.env.read_txn().unwrap();
    assert_eq!(store.operations.len(&transaction).unwrap(), 1000);
    assert_eq!(store.contents.len(&transaction).unwrap(), 1001);
    let digest = blake3::hash(b"shared");
    assert_eq!(
        store
            .references
            .get(&transaction, digest.as_bytes())
            .unwrap(),
        Some(1000)
    );
}
#[test]
fn aborted_write_does_not_leave_orphaned_content() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    let mut transaction = store.env.write_txn().unwrap();
    store
        .retain_content(&mut transaction, &present("uncommitted"))
        .unwrap();
    transaction.abort();
    let reader = store.env.read_txn().unwrap();
    assert!(store.contents.is_empty(&reader).unwrap());
    assert!(store.references.is_empty(&reader).unwrap());
}
