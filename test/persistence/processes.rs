use crate::operation::OperationService;
use core::time::Duration;
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::Instant,
};
pub(super) fn spawn(directory: &Path, mode: &str, worker: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", worker, "--nocapture"])
        .env("OPENPATCH_TEST_DIRECTORY", directory)
        .env("OPENPATCH_TEST_MODE", mode)
        .stdin(Stdio::null())
        .spawn()
        .unwrap()
}
pub(super) fn wait_ready(child: &mut Child, path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !path.exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "worker exited before readiness"
        );
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("worker readiness timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn simultaneous_processes_share_history_and_serialize_file_edits() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("counter.txt"), "").unwrap();
    let service = OperationService::open(&directory.path().join("history.lmdb")).unwrap();
    let mut children = (0_u8..4)
        .map(|index| {
            spawn(
                directory.path(),
                &index.to_string(),
                "service::operation::store::tests::processes::writer",
            )
        })
        .collect::<Vec<_>>();
    for (index, child) in children.iter_mut().enumerate() {
        wait_ready(child, &directory.path().join(format!("ready-{index}")));
    }
    fs::write(directory.path().join("start"), "").unwrap();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let ids = (0_u8..4)
        .map(|index| fs::read_to_string(directory.path().join(format!("result-{index}"))).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        fs::read_to_string(directory.path().join("counter.txt"))
            .unwrap()
            .lines()
            .count(),
        4
    );
    let undone = service.undo(&ids);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(
        fs::read_to_string(directory.path().join("counter.txt")).unwrap(),
        ""
    );
}
#[test]
fn already_running_service_recovers_another_process_before_accepting_edits() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(root.join("source.txt"), "source").unwrap();
    fs::write(root.join("destination.txt"), "displaced").unwrap();
    let service = OperationService::open(&root.join("history.lmdb")).unwrap();
    super::interruption::crash(root, "destination");
    let output = service.apply(&format!(
        "*** Begin Patch\n*** Add File: {}\n+next\n*** End Patch",
        root.join("following.txt").display(),
    ));
    assert!(output.succeeded(), "{}", output.render());
    assert!(!root.join("source.txt").exists());
    assert_eq!(
        fs::read_to_string(root.join("destination.txt")).unwrap(),
        "source"
    );
    let recovered_id = fs::read_to_string(root.join("id")).unwrap();
    let undone = service.undo(&[recovered_id]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(
        fs::read_to_string(root.join("source.txt")).unwrap(),
        "source"
    );
}
#[test]
fn writer() {
    let Some(directory) = std::env::var_os("OPENPATCH_TEST_DIRECTORY") else {
        return;
    };
    let root = std::path::PathBuf::from(directory);
    let index = std::env::var("OPENPATCH_TEST_MODE").unwrap();
    let service = OperationService::open(&root.join("history.lmdb")).unwrap();
    fs::write(root.join(format!("ready-{index}")), "").unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !root.join("start").exists() {
        assert!(Instant::now() < deadline, "start barrier timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = service.apply(&format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n+{index}\n*** End Patch",
        root.join("counter.txt").display(),
    ));
    assert!(output.succeeded(), "{}", output.render());
    let rendered = output.render();
    let (_, suffix) = rendered.split_once("<ID>\n").unwrap();
    let (id, _) = suffix.split_once("\n</ID>").unwrap();
    fs::write(root.join(format!("result-{index}")), id).unwrap();
}
