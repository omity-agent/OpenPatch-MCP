use super::{HistoryStore, mutation, present, record, store};
use crate::operation::model::{Mutation, OperationKind};
#[test]
fn damaged_raw_and_compressed_content_is_rejected() {
    for contents in [String::from("x"), "compress me\n".repeat(512)] {
        let directory = tempfile::tempdir().unwrap();
        let store = store(&directory);
        let mut mutation = mutation();
        mutation.changes.first_mut().unwrap().after = present(&contents);
        let id = record(&store, &mutation);
        let digest = blake3::hash(contents.as_bytes());
        let raw = store.contents.remap_data_type::<heed::types::Bytes>();
        let mut writer = store.env.write_txn().unwrap();
        let mut bytes = raw
            .get(&writer, digest.as_bytes())
            .unwrap()
            .unwrap()
            .to_vec();
        *bytes.last_mut().unwrap() ^= 0xff;
        raw.put(&mut writer, digest.as_bytes(), &bytes).unwrap();
        writer.commit().unwrap();
        store.load(id).unwrap_err();
    }
}
#[test]
fn non_unicode_paths_round_trip_without_loss() {
    let directory = tempfile::tempdir().unwrap();
    let store = store(&directory);
    #[cfg(windows)]
    let path = {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[0xd800, 0x61])
    };
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![0xff, b'a'])
    };
    let mutation = Mutation::single(
        OperationKind::Edit,
        path.into(),
        present("before"),
        present("after"),
    );
    let id = record(&store, &mutation);
    assert_eq!(store.load(id).unwrap().changes, mutation.changes);
}
#[test]
fn capacity_failure_preserves_history_and_can_be_resolved_by_reopening_larger() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("capacity.lmdb");
    let small = HistoryStore::open_capacity(&path, 64 * 1024).unwrap();
    let original = record(&small, &mutation());
    let contents = core::iter::repeat_with(|| char::from(rand::random_range(b'a'..=b'z')))
        .take(128 * 1024)
        .collect::<String>();
    let mut large = mutation();
    large.changes.first_mut().unwrap().after = present(&contents);
    let error = small
        .prepare(core::slice::from_ref(&large), &[], None)
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<heed::Error>(),
        Some(&heed::Error::Mdb(heed::MdbError::MapFull))
    ));
    assert!(small.recovery().unwrap().is_none());
    small.load(original).unwrap();
    drop(small);
    let expanded = HistoryStore::open_capacity(&path, 4 * 1024 * 1024).unwrap();
    record(&expanded, &large);
    expanded.load(original).unwrap();
}
