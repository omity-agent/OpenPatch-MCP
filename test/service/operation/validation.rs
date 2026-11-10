use super::super::{apply::files, model::FileState, store::Transition};
use std::fs;
#[test]
fn exact_validation_only_publishes_unchanged_files() {
    for (original, concurrent) in [
        (Some(format!("{}é-tail", "a".repeat(32 * 1_024))), None),
        (Some("a".repeat(32 * 1_024)), Some("b".repeat(32 * 1_024))),
        (None, Some(String::from("concurrent"))),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("target.txt");
        if let Some(contents) = original {
            fs::write(&path, contents).unwrap();
        }
        let transitions = [Transition {
            path: path.clone(),
            before: files::snapshot(&path, "snapshot failed").unwrap(),
            after: FileState::present(String::from("updated")),
        }];
        if let Some(contents) = concurrent {
            fs::write(&path, &contents).unwrap();
            let error = files::verify(&transitions).unwrap_err();
            assert!(error.to_string().contains("file changed concurrently"));
            assert_eq!(fs::read_to_string(path).unwrap(), contents);
        } else {
            files::verify(&transitions).unwrap();
            files::publish(&transitions).unwrap();
            assert_eq!(fs::read_to_string(path).unwrap(), "updated");
        }
    }
}
