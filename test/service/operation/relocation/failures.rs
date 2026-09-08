use super::{ids_from, service};
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
