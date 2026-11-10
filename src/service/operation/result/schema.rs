use rmcp::schemars::Schema;
pub(super) fn remove_integer_format(schema: &mut Schema) {
    schema.remove("format");
}
pub(super) fn require_success_fields(schema: &mut Schema) {
    schema.insert(
        String::from("required"),
        required_fields(&["kind", "path", "before", "after", "id", "undoOf"]),
    );
}
pub(super) fn require_failure_fields(schema: &mut Schema) {
    schema.insert(
        String::from("required"),
        required_fields(&["operation", "undoId", "reason"]),
    );
}
fn required_fields(fields: &[&str]) -> rmcp::serde_json::Value {
    rmcp::serde_json::Value::Array(fields.iter().map(|field| (*field).into()).collect())
}
