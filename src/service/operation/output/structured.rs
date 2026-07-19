use super::{Failure, FileStats, OperationKind, OperationOutput, Success};
use rmcp::schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PatchToolOutput {
    succeeded: bool,
    successes: Vec<SuccessfulOperation>,
    failures: Vec<OperationFailure>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename_all = "camelCase", deny_unknown_fields)]
struct SuccessfulOperation {
    kind: OutputOperationKind,
    path: String,
    #[schemars(required)]
    before: Option<FileStatistics>,
    #[schemars(required)]
    after: Option<FileStatistics>,
    uuid: String,
    #[schemars(required)]
    undo_of: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct FileStatistics {
    #[serde(rename = "lineCount")]
    #[schemars(rename = "lineCount")]
    line_count: usize,
    #[serde(rename = "characterCount")]
    #[schemars(rename = "characterCount")]
    character_count: usize,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename_all = "camelCase", deny_unknown_fields)]
struct OperationFailure {
    #[schemars(required)]
    operation: Option<FailedOperation>,
    #[schemars(required)]
    undo_uuid: Option<String>,
    reason: String,
}
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct FailedOperation {
    kind: OutputOperationKind,
    path: String,
}
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
#[schemars(rename_all = "UPPERCASE")]
enum OutputOperationKind {
    Add,
    Edit,
    Delete,
}
impl OperationOutput {
    pub(crate) fn structured(&self) -> rmcp::serde_json::Value {
        rmcp::serde_json::json!(PatchToolOutput::from(self))
    }
}
impl From<&OperationOutput> for PatchToolOutput {
    fn from(output: &OperationOutput) -> Self {
        Self {
            succeeded: output.succeeded(),
            successes: output
                .successes
                .iter()
                .map(SuccessfulOperation::from)
                .collect(),
            failures: output.failures.iter().map(OperationFailure::from).collect(),
        }
    }
}
impl From<&Success> for SuccessfulOperation {
    fn from(success: &Success) -> Self {
        Self {
            kind: success.kind.into(),
            path: success.path.display().to_string(),
            before: success.before.map(FileStatistics::from),
            after: success.after.map(FileStatistics::from),
            uuid: success.uuid.to_string(),
            undo_of: success.undo_of.map(|uuid| uuid.to_string()),
        }
    }
}
impl From<FileStats> for FileStatistics {
    fn from(statistics: FileStats) -> Self {
        Self {
            line_count: statistics.line_count,
            character_count: statistics.character_count,
        }
    }
}
impl From<&Failure> for OperationFailure {
    fn from(failure: &Failure) -> Self {
        Self {
            operation: failure.operation.as_ref().map(|operation| FailedOperation {
                kind: operation.0.into(),
                path: operation.1.display().to_string(),
            }),
            undo_uuid: failure.undo_uuid.clone(),
            reason: failure.reason.clone(),
        }
    }
}
impl From<OperationKind> for OutputOperationKind {
    fn from(kind: OperationKind) -> Self {
        match kind {
            OperationKind::Add => Self::Add,
            OperationKind::Edit => Self::Edit,
            OperationKind::Delete => Self::Delete,
        }
    }
}
