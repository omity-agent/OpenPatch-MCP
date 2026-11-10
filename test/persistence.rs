use super::HistoryStore;
use crate::operation::model::{FileState, Mutation, OperationId, OperationKind};
use std::{
    fs,
    path::{Path, PathBuf},
};
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
fn move_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("source.txt"), "source").unwrap();
    fs::write(directory.path().join("destination.txt"), "displaced").unwrap();
    directory
}
fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).unwrap()
}
fn assert_file(root: &Path, name: &str, expected: &str) {
    assert_eq!(read(root, name), expected, "{name}");
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
    let references = store
        .references
        .get(&transaction, digest.as_bytes())
        .unwrap();
    assert_eq!(references, Some(1000));
}
