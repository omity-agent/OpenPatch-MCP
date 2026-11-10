use rmcp::serde_json::{Value, json};
use std::path::Path;
pub(super) fn verify_success(structured: &Value, target: &Path, line_count: u64) {
    assert_eq!(structured["succeeded"], true);
    let success = structured.pointer("/successes/0").unwrap();
    let mut fields = success
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    fields.sort_unstable();
    assert_eq!(fields, ["after", "before", "id", "kind", "path", "undoOf"]);
    assert_eq!(success["kind"], "EDIT");
    assert_eq!(
        success["path"].as_str().unwrap(),
        target.display().to_string()
    );
    for state in ["before", "after"] {
        assert_eq!(
            *success.pointer(&format!("/{state}/lineCount")).unwrap(),
            line_count
        );
    }
    let id = success["id"].as_str().unwrap();
    assert_eq!(
        crate::operation::model::OperationId::parse(id)
            .unwrap()
            .to_string(),
        id
    );
    assert!(success["undoOf"].is_null());
    assert_eq!(structured["failures"], json!([]));
}
pub(super) fn verify_failure(structured: &Value) {
    assert_eq!(structured["succeeded"], false);
    assert_eq!(structured["successes"], json!([]));
    assert_eq!(
        structured.pointer("/failures/0").unwrap(),
        &json!({"operation": null, "undoId": null, "reason": "patch must not be empty"})
    );
}
