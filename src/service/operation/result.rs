mod render;
use super::model::{OperationId, OperationKind};
use rmcp::schemars::{JsonSchema, Schema};
use sonic_rs::Serialize;
use std::path::PathBuf;
pub(crate) type PatchToolOutput = OperationOutput;
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(
    rename = "PatchToolOutput",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(crate) struct OperationOutput {
    succeeded: bool,
    successes: Vec<Success>,
    failures: Vec<Failure>,
}
impl Default for OperationOutput {
    fn default() -> Self {
        Self {
            succeeded: true,
            successes: Vec::new(),
            failures: Vec::new(),
        }
    }
}
impl OperationOutput {
    pub(super) fn failed(reason: String) -> Self {
        Self {
            succeeded: false,
            successes: Vec::new(),
            failures: vec![Failure::global(reason)],
        }
    }
    pub(super) fn push_success(&mut self, success: Success) {
        self.successes.push(success);
    }
    pub(super) fn push_failure(&mut self, failure: Failure) {
        self.succeeded = false;
        self.failures.push(failure);
    }
    pub(crate) const fn succeeded(&self) -> bool {
        self.succeeded
    }
    pub(crate) fn render(&self) -> String {
        render::operations(self)
    }
    pub(crate) fn structured(&self) -> sonic_rs::Value {
        sonic_rs::json!(self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(rename = "FileStatistics", deny_unknown_fields)]
pub(super) struct FileStats {
    #[serde(rename = "lineCount")]
    # [schemars (rename = "lineCount" , transform = remove_integer_format)]
    line_count: usize,
    #[serde(rename = "characterCount")]
    # [schemars (rename = "characterCount" , transform = remove_integer_format)]
    character_count: usize,
}
impl FileStats {
    pub(super) fn from_contents(contents: &str) -> Self {
        Self {
            line_count: crate::text::line_count(contents),
            character_count: crate::text::character_count(contents),
        }
    }
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
# [schemars (rename = "SuccessfulOperation" , rename_all = "camelCase" , deny_unknown_fields , transform = require_success_fields)]
pub(super) struct Success {
    kind: OperationKind,
    path: String,
    before: Option<FileStats>,
    after: Option<FileStats>,
    uuid: String,
    undo_of: Option<String>,
}
impl Success {
    pub(super) fn new(
        kind: OperationKind,
        uuid: OperationId,
        undo_of: Option<OperationId>,
        path: PathBuf,
        before: Option<FileStats>,
        after: Option<FileStats>,
    ) -> Self {
        Self {
            kind,
            path: display_path(path),
            before,
            after,
            uuid: uuid.to_string(),
            undo_of: undo_of.map(|value| value.to_string()),
        }
    }
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
# [schemars (rename = "OperationFailure" , rename_all = "camelCase" , deny_unknown_fields , transform = require_failure_fields)]
pub(super) struct Failure {
    operation: Option<FailedOperation>,
    undo_uuid: Option<String>,
    reason: String,
}
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(rename = "FailedOperation", deny_unknown_fields)]
struct FailedOperation {
    kind: OperationKind,
    path: String,
}
impl Failure {
    pub(super) fn file(kind: OperationKind, path: PathBuf, reason: String) -> Self {
        Self {
            operation: Some(FailedOperation {
                kind,
                path: display_path(path),
            }),
            undo_uuid: None,
            reason,
        }
    }
    pub(super) const fn undo(uuid: String, reason: String) -> Self {
        Self {
            operation: None,
            undo_uuid: Some(uuid),
            reason,
        }
    }
    pub(super) const fn global(reason: String) -> Self {
        Self {
            operation: None,
            undo_uuid: None,
            reason,
        }
    }
}
fn remove_integer_format(schema: &mut Schema) {
    schema.remove("format");
}
fn require_success_fields(schema: &mut Schema) {
    schema.insert(
        String::from("required"),
        required_fields(&["kind", "path", "before", "after", "uuid", "undoOf"]),
    );
}
fn require_failure_fields(schema: &mut Schema) {
    schema.insert(
        String::from("required"),
        required_fields(&["operation", "undoUuid", "reason"]),
    );
}
fn required_fields(fields: &[&str]) -> rmcp::serde_json::Value {
    rmcp::serde_json::Value::Array(fields.iter().map(|field| (*field).into()).collect())
}
fn display_path(path: PathBuf) -> String {
    path.into_os_string().to_string_lossy().into_owned()
}
