use rmcp::serde_json::Value;
use std::path::Path;
pub(super) fn verify_success(structured: &Value, target_path: &Path) {
    let result = structured.as_object().unwrap();
    assert_eq!(result.get("succeeded").and_then(Value::as_bool), Some(true));
    let success = result
        .get("successes")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_object()
        .unwrap();
    let mut success_fields = success.keys().map(String::as_str).collect::<Vec<_>>();
    success_fields.sort_unstable();
    assert_eq!(
        success_fields,
        ["after", "before", "kind", "path", "undoOf", "uuid"]
    );
    assert_eq!(success.get("kind").and_then(Value::as_str), Some("EDIT"));
    assert_eq!(
        success.get("path").unwrap().as_str().unwrap(),
        target_path.display().to_string()
    );
    assert_eq!(
        success
            .get("before")
            .unwrap()
            .get("lineCount")
            .unwrap()
            .as_u64(),
        Some(1)
    );
    assert_eq!(
        success
            .get("after")
            .unwrap()
            .get("lineCount")
            .unwrap()
            .as_u64(),
        Some(1)
    );
    assert!(success.get("uuid").unwrap().is_string());
    assert!(success.get("undoOf").unwrap().is_null());
    let no_failures: &[Value] = &[];
    assert_eq!(
        result
            .get("failures")
            .unwrap()
            .as_array()
            .unwrap()
            .as_slice(),
        no_failures
    );
}
pub(super) fn verify_failure(structured: &Value) {
    let result = structured.as_object().unwrap();
    assert_eq!(
        result.get("succeeded").and_then(Value::as_bool),
        Some(false)
    );
    let no_successes: &[Value] = &[];
    assert_eq!(
        result
            .get("successes")
            .unwrap()
            .as_array()
            .unwrap()
            .as_slice(),
        no_successes
    );
    let failure = result
        .get("failures")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_object()
        .unwrap();
    assert!(failure.get("operation").unwrap().is_null());
    assert!(failure.get("undoUuid").unwrap().is_null());
    assert_eq!(
        failure.get("reason").and_then(Value::as_str),
        Some("patch must not be empty")
    );
}
