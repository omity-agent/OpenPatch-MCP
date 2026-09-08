use super::{ids_from, service};
use crate::operation::result::OperationOutput;
use std::{fs, path::Path};
#[path = "relocation/failures.rs"]
mod failures;
#[path = "relocation/interleaving.rs"]
mod interleaving;
fn assert_operations(output: &OperationOutput, expected: &[(&str, &Path)]) -> Vec<String> {
    let rendered = output.render();
    assert!(output.succeeded(), "{rendered}");
    let ids = ids_from(&rendered);
    assert_eq!(ids.len(), expected.len(), "{rendered}");
    let structured = output.structured();
    for (index, &(kind, path)) in expected.iter().enumerate() {
        assert_eq!(structured["successes"][index]["kind"], kind);
        assert_eq!(
            structured["successes"][index]["path"],
            path.display().to_string()
        );
        assert!(rendered.contains(&format!("<{kind}>\n{}", path.display())));
        if kind == "MOVE" {
            assert_eq!(
                structured["successes"][index]["before"],
                structured["successes"][index]["after"]
            );
        }
    }
    ids
}
#[test]
fn same_path_move_with_content_changes_remains_edit() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old\n").unwrap();
    let patch = format!(
        "*** Begin Patch
*** Update File: {}
*** Move to: {}
@@
-old
+new
*** End Patch",
        target.display(),
        target.display()
    );
    let applied = service.apply(&patch);
    assert!(applied.succeeded(), "{}", applied.render());
    assert_eq!(applied.structured()["successes"][0]["kind"], "EDIT");
    assert!(!applied.render().contains("<MOVE>"));
    assert_eq!(fs::read_to_string(target).unwrap(), "new\n");
}
