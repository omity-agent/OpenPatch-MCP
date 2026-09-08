use super::{HistoryStore, present, processes, store};
use crate::operation::{
    OperationService,
    apply::{durable, files},
    model::{FileState, Mutation, OperationId},
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
#[test]
fn crashes_at_each_publication_boundary_roll_forward_once() {
    for stage in ["prepared", "destination", "published", "finished"] {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("source.txt"), "source").unwrap();
        fs::write(directory.path().join("destination.txt"), "displaced").unwrap();
        crash(directory.path(), stage);
        let id = fs::read_to_string(directory.path().join("id")).unwrap();
        let service = OperationService::open(&directory.path().join("history.lmdb")).unwrap();
        assert!(!directory.path().join("source.txt").exists());
        assert_eq!(
            fs::read_to_string(directory.path().join("destination.txt")).unwrap(),
            "source"
        );
        let store = store(&directory);
        assert!(store.recovery().unwrap().is_none());
        let reader = store.env.read_txn().unwrap();
        assert_eq!(store.operations.len(&reader).unwrap(), 1);
        drop(reader);
        let undone = service.undo(&[id]);
        assert!(undone.succeeded(), "{}", undone.render());
        assert_eq!(
            fs::read_to_string(directory.path().join("source.txt")).unwrap(),
            "source"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("destination.txt")).unwrap(),
            "displaced"
        );
    }
}
#[test]
fn recovery_conflict_preserves_intent_and_all_external_bytes() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("source.txt"), "source").unwrap();
    fs::write(directory.path().join("destination.txt"), "displaced").unwrap();
    crash(directory.path(), "destination");
    fs::write(directory.path().join("source.txt"), "external").unwrap();
    let store = store(&directory);
    let guard = store.lock().unwrap();
    assert!(durable::resume(&store).is_err());
    assert!(store.recovery().unwrap().is_some());
    assert_eq!(
        fs::read_to_string(directory.path().join("source.txt")).unwrap(),
        "external"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("destination.txt")).unwrap(),
        "source"
    );
    fs::write(directory.path().join("source.txt"), "source").unwrap();
    durable::resume(&store).unwrap();
    assert!(store.recovery().unwrap().is_none());
    drop(guard);
}
#[test]
fn interrupted_undo_consumes_original_only_when_finalized() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("source.txt"), "source").unwrap();
    fs::write(directory.path().join("destination.txt"), "displaced").unwrap();
    crash(directory.path(), "undo");
    let store = store(&directory);
    let original =
        OperationId::parse(&fs::read_to_string(directory.path().join("id")).unwrap()).unwrap();
    store.load(original).unwrap();
    let guard = store.lock().unwrap();
    durable::resume(&store).unwrap();
    assert!(
        store
            .load(original)
            .unwrap_err()
            .to_string()
            .contains("already undone")
    );
    durable::resume(&store).unwrap();
    let reader = store.env.read_txn().unwrap();
    assert_eq!(store.operations.len(&reader).unwrap(), 2);
    drop(reader);
    drop(guard);
    let service = OperationService::open(&directory.path().join("history.lmdb")).unwrap();
    let undo_id = fs::read_to_string(directory.path().join("undo-id")).unwrap();
    assert!(service.undo(&[undo_id]).succeeded());
    assert!(!directory.path().join("source.txt").exists());
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
    let transitions = mutation
        .changes
        .iter()
        .map(|change| Transition {
            path: change.path.clone(),
            before: change.before.clone(),
            after: change.after.clone(),
        })
        .collect::<Vec<_>>();
    let ids = store.prepare(&[mutation], &transitions, None).unwrap();
    let id = *ids.first().unwrap();
    fs::write(root.join("id"), id.to_string()).unwrap();
    if mode != "prepared" {
        let destination = transitions.first().unwrap();
        files::write_state(&destination.path, &destination.after).unwrap();
    }
    if matches!(mode.as_str(), "published" | "finished" | "undo") {
        files::publish(&transitions).unwrap();
    }
    if matches!(mode.as_str(), "finished" | "undo") {
        store.finish().unwrap();
    }
    if mode == "undo" {
        let inverse = crate::operation::model::Mutation {
            kind: crate::operation::model::OperationKind::Move,
            display_path: root.join("source.txt"),
            changes: smallvec::smallvec![
                crate::operation::model::PathChange {
                    role: crate::operation::model::PathRole::Destination,
                    path: root.join("source.txt"),
                    before: FileState::Missing,
                    after: present("source"),
                },
                crate::operation::model::PathChange {
                    role: crate::operation::model::PathRole::Source,
                    path: root.join("destination.txt"),
                    before: present("source"),
                    after: present("displaced"),
                },
            ],
        };
        let restoring = inverse
            .changes
            .iter()
            .map(|change| Transition {
                path: change.path.clone(),
                before: change.before.clone(),
                after: change.after.clone(),
            })
            .collect::<Vec<_>>();
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
