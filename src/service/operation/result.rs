mod explanation;
mod labels;
mod render;
mod schema;
use super::model::{OperationId, OperationKind};
use explanation::Diagnostic;
pub(crate) use explanation::PathContext;
use rmcp::schemars::JsonSchema;
use schema::{remove_integer_format, require_failure_fields, require_success_fields};
use sonic_rs::Serialize;
use std::path::{Path, PathBuf};
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
    pub(super) fn failed(reason: impl Into<Diagnostic>) -> Self {
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
    pub(super) fn finish(mut self) -> Self {
        let labels = labels::PathLabels::new(&self);
        for success in &mut self.successes {
            success.path = labels.display(Path::new(&success.path));
        }
        for failure in &mut self.failures {
            if let Some(operation) = failure.operation.as_mut() {
                operation.path = labels.display(Path::new(&operation.path));
            }
            failure.reason.finish(&labels);
        }
        self
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
    # [schemars (regex (pattern = super :: model :: ID_PATTERN))]
    id: String,
    # [schemars (regex (pattern = super :: model :: ID_PATTERN))]
    undo_of: Option<String>,
}
impl Success {
    pub(super) fn new(
        kind: OperationKind,
        id: OperationId,
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
            id: id.to_string(),
            undo_of: undo_of.map(|value| value.to_string()),
        }
    }
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
# [schemars (rename = "OperationFailure" , rename_all = "camelCase" , deny_unknown_fields , transform = require_failure_fields)]
pub(super) struct Failure {
    operation: Option<FailedOperation>,
    undo_id: Option<String>,
    #[schemars(with = "String")]
    reason: Diagnostic,
}
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(rename = "FailedOperation", deny_unknown_fields)]
struct FailedOperation {
    kind: OperationKind,
    path: String,
}
impl Failure {
    pub(super) fn file(kind: OperationKind, path: PathBuf, reason: impl Into<Diagnostic>) -> Self {
        Self {
            operation: Some(FailedOperation {
                kind,
                path: display_path(path),
            }),
            undo_id: None,
            reason: reason.into(),
        }
    }
    pub(super) fn undo(id: String, reason: impl Into<Diagnostic>) -> Self {
        Self {
            operation: None,
            undo_id: Some(id),
            reason: reason.into(),
        }
    }
    pub(super) fn global(reason: impl Into<Diagnostic>) -> Self {
        Self {
            operation: None,
            undo_id: None,
            reason: reason.into(),
        }
    }
}
fn display_path(path: PathBuf) -> String {
    path.into_os_string().to_string_lossy().into_owned()
}
