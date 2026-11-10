use super::{HistoryStore, assert_file, move_directory, present, processes, read, store};
use crate::operation::{
    OperationService,
    apply::{durable, files},
    model::{Mutation, OperationId, PathRole},
    store::Transition,
};
use core::time::Duration;
use std::{fs, path::Path};
pub(super) fn crash(directory: &Path, mode: &str) {
    let mut child = processes::spawn(
        directory,
        mode,
        "service::operation::store::tests::interruption::worker",
    );
    processes::wait_ready(&mut child, &directory.join("ready"));
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
}
fn transitions(mutation: &Mutation) -> Vec<Transition> {
    mutation
        .changes
        .iter()
        .map(|change| Transition {
            path: change.path.clone(),
            before: change.before.clone(),
            after: change.after.clone(),
        })
        .collect()
}
#[test]
fn crashes_at_each_publication_boundary_roll_forward_once() {
    for stage in ["prepared", "destination", "published", "finished"] {
        let directory = move_directory();
        let root = directory.path();
        crash(root, stage);
        let id = read(root, "id");
        let service = OperationService::open(&root.join("history.lmdb")).unwrap();
        assert!(!root.join("source.txt").exists());
        assert_file(root, "destination.txt", "source");
        let store = store(&directory);
        assert!(store.recovery().unwrap().is_none());
        let reader = store.env.read_txn().unwrap();
        assert_eq!(store.operations.len(&reader).unwrap(), 1);
        drop(reader);
        let undone = service.undo(&[id]);
        assert!(undone.succeeded(), "{}", undone.render());
        assert_file(root, "source.txt", "source");
        assert_file(root, "destination.txt", "displaced");
    }
}
#[test]
fn recovery_conflict_preserves_intent_and_all_external_bytes() {
    let directory = move_directory();
    let root = directory.path();
    crash(root, "destination");
    fs::write(root.join("source.txt"), "external").unwrap();
    let store = store(&directory);
    let guard = store.lock().unwrap();
    assert!(durable::resume(&store).is_err());
    assert!(store.recovery().unwrap().is_some());
    assert_file(root, "source.txt", "external");
    assert_file(root, "destination.txt", "source");
    fs::write(root.join("source.txt"), "source").unwrap();
    durable::resume(&store).unwrap();
    assert!(store.recovery().unwrap().is_none());
    drop(guard);
}
#[test]
fn interrupted_undo_consumes_original_only_when_finalized() {
    let directory = move_directory();
    let root = directory.path();
    crash(root, "undo");
    let store = store(&directory);
    let original = OperationId::parse(&read(root, "id")).unwrap();
    store.load(original).unwrap();
    let guard = store.lock().unwrap();
    durable::resume(&store).unwrap();
    let error = store.load(original).unwrap_err().to_string();
    assert!(error.contains("already undone"));
    durable::resume(&store).unwrap();
    let reader = store.env.read_txn().unwrap();
    assert_eq!(store.operations.len(&reader).unwrap(), 2);
    drop(reader);
    drop(guard);
    let service = OperationService::open(&root.join("history.lmdb")).unwrap();
    let undo_id = read(root, "undo-id");
    assert!(service.undo(&[undo_id]).succeeded());
    assert!(!root.join("source.txt").exists());
}
#[test]
fn worker() {
    let Some(directory) = std::env::var_os("OPENPATCH_TEST_DIRECTORY") else {
        return;
    };
    let root = std::path::PathBuf::from(directory);
    let mode = std::env::var("OPENPATCH_TEST_MODE").unwrap();
    let store = HistoryStore::open(&root.join("history.lmdb")).unwrap();
    let _guard = store.lock().unwrap();
    let mutation = Mutation::moved(
        root.join("source.txt"),
        root.join("destination.txt"),
        present("source"),
        present("displaced"),
    );
    let published = transitions(&mutation);
    let ids = store
        .prepare(core::slice::from_ref(&mutation), &published, None)
        .unwrap();
    let id = *ids.first().unwrap();
    fs::write(root.join("id"), id.to_string()).unwrap();
    if mode != "prepared" {
        let destination = published.first().unwrap();
        files::write_state(&destination.path, &destination.after).unwrap();
    }
    if matches!(mode.as_str(), "published" | "finished" | "undo") {
        files::publish(&published).unwrap();
    }
    if matches!(mode.as_str(), "finished" | "undo") {
        store.finish().unwrap();
    }
    if mode == "undo" {
        let mut inverse = mutation;
        inverse.display_path = root.join("source.txt");
        inverse.changes.reverse();
        for change in &mut inverse.changes {
            core::mem::swap(&mut change.before, &mut change.after);
            change.role = match change.role {
                PathRole::Source => PathRole::Destination,
                PathRole::Destination => PathRole::Source,
                PathRole::Single => panic!("move must have paired changes"),
            };
        }
        let restoring = transitions(&inverse);
        let undo_ids = store.prepare(&[inverse], &restoring, Some(id)).unwrap();
        fs::write(root.join("undo-id"), undo_ids.first().unwrap().to_string()).unwrap();
        files::publish(&restoring).unwrap();
    }
    let mut transaction = store.env.write_txn().unwrap();
    if mode == "undo" {
        store.finalize(&mut transaction).unwrap();
    }
    fs::write(root.join("ready"), "").unwrap();
    std::thread::sleep(Duration::from_secs(60));
    panic!("parent failed to terminate crash worker");
}
