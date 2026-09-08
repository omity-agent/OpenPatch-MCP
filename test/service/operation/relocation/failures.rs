use super::{ids_from, service};
use diesel::connection::SimpleConnection as _;
use std::fs;
#[test]
fn chunk_failures_are_edits_and_do_not_move_the_file() {
    for contents in [None, Some("original\n")] {
        let directory = tempfile::tempdir().unwrap();
        let service = service(&directory);
        let source = directory.path().join("source.txt");
        let destination = directory.path().join("destination.txt");
        if let Some(original) = contents {
            fs::write(&source, original).unwrap();
        }
        let failed = service . apply (& format ! ("*** Begin Patch\n*** Update File: {}\n*** Move to: {}\n@@\n-absent\n+replacement\n*** End Patch" , source . display () , destination . display ())) ;
        assert!(!failed.succeeded());
        assert_eq!(ids_from(&failed.render()), Vec::<String>::new());
        let structured = failed.structured();
        let (kind, path) = if contents.is_some() {
            ("EDIT", &source)
        } else {
            ("MOVE", &destination)
        };
        assert!(failed.render().contains(&format!("<{kind}>")));
        assert_eq!(structured["failures"][0]["operation"]["kind"], kind);
        assert_eq!(
            structured["failures"][0]["operation"]["path"],
            path.display().to_string()
        );
        assert!(!destination.exists());
        if let Some(original) = contents {
            assert_eq!(fs::read_to_string(source).unwrap(), original);
        } else {
            assert!(!source.exists());
        }
    }
}
#[test]
fn failed_steps_do_not_apply_dependents_but_allow_unrelated_files() {
    for move_first in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let service = service(&directory);
        let source = directory.path().join("source.txt");
        let destination = directory.path().join("destination.txt");
        let unrelated = directory.path().join("unrelated.txt");
        fs::write(&source, "old\n").unwrap();
        let kind = if move_first { 3_i64 } else { 1_i64 };
        service
            .history
            .lock_connection()
            .unwrap()
            .batch_execute(&format!(
                "CREATE TRIGGER reject_step BEFORE INSERT ON operations
                 WHEN NEW.kind = {kind} BEGIN SELECT RAISE(ABORT, 'forced step failure'); END;"
            ))
            .unwrap();
        let body = if move_first {
            format!(
                "*** Update File: {}\n*** Move to: {}\n*** Update File: {}\n@@\n-old\n+updated",
                source.display(),
                destination.display(),
                destination.display()
            )
        } else {
            format!(
                "*** Update File: {}\n*** Move to: {}\n@@\n-old\n+updated",
                source.display(),
                destination.display()
            )
        };
        let applied = service.apply(&format!(
            "*** Begin Patch\n{body}\n*** Add File: {}\n+unrelated\n*** End Patch",
            unrelated.display()
        ));
        assert!(!applied.succeeded());
        assert_eq!(ids_from(&applied.render()).len(), 1);
        assert_eq!(applied.structured()["successes"][0]["kind"], "ADD");
        assert!(applied.render().contains("<EDIT>"));
        assert!(applied.render().contains("<MOVE>"));
        assert_eq!(fs::read_to_string(source).unwrap(), "old\n");
        assert!(!destination.exists());
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "unrelated\n");
    }
}
