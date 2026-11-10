use super::{ids_from, service};
use crate::operation::result::OperationOutput;
use std::path::Path;
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
        let label = path.file_name().unwrap().to_string_lossy();
        assert_eq!(structured["successes"][index]["kind"], kind);
        assert_eq!(structured["successes"][index]["path"], label.as_ref());
        assert!(rendered.contains(&format!("<{kind}>\n{label}\n")));
        if kind == "MOVE" {
            assert_eq!(
                structured["successes"][index]["before"],
                structured["successes"][index]["after"]
            );
        }
    }
    ids
}
