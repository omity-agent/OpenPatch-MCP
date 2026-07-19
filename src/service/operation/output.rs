mod structured;
use super::model::{OperationId, OperationKind};
use core::fmt::Write as _;
use std::path::{Path, PathBuf};
pub(crate) use structured::PatchToolOutput;
#[derive(Debug, Default)]
pub(crate) struct OperationOutput {
    successes: Vec<Success>,
    failures: Vec<Failure>,
}
impl OperationOutput {
    pub(super) fn failed(reason: String) -> Self {
        Self {
            successes: Vec::new(),
            failures: vec![Failure::global(reason)],
        }
    }
    pub(super) fn push_success(&mut self, success: Success) {
        self.successes.push(success);
    }
    pub(super) fn push_failure(&mut self, failure: Failure) {
        self.failures.push(failure);
    }
    pub(crate) const fn succeeded(&self) -> bool {
        self.failures.is_empty()
    }
    pub(crate) fn render(&self) -> String {
        let mut output = String::with_capacity((self.successes.len() + self.failures.len()) * 192);
        if !self.successes.is_empty() {
            output.push_str("<SUCCEEDED>\n");
            for success in &self.successes {
                push_success(&mut output, success);
            }
            output.push_str("</SUCCEEDED>");
        }
        if !self.failures.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("<FAILED>\n");
            for failure in &self.failures {
                push_failure(&mut output, failure);
            }
            output.push_str("</FAILED>");
        }
        output
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileStats {
    line_count: usize,
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
#[derive(Debug)]
pub(super) struct Success {
    kind: OperationKind,
    uuid: OperationId,
    undo_of: Option<OperationId>,
    path: PathBuf,
    before: Option<FileStats>,
    after: Option<FileStats>,
}
impl Success {
    pub(super) const fn new(
        kind: OperationKind,
        uuid: OperationId,
        undo_of: Option<OperationId>,
        path: PathBuf,
        before: Option<FileStats>,
        after: Option<FileStats>,
    ) -> Self {
        Self {
            kind,
            uuid,
            undo_of,
            path,
            before,
            after,
        }
    }
}
#[derive(Debug)]
pub(super) struct Failure {
    operation: Option<(OperationKind, PathBuf)>,
    undo_uuid: Option<String>,
    reason: String,
}
impl Failure {
    pub(super) const fn file(kind: OperationKind, path: PathBuf, reason: String) -> Self {
        Self {
            operation: Some((kind, path)),
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
    const fn global(reason: String) -> Self {
        Self {
            operation: None,
            undo_uuid: None,
            reason,
        }
    }
}
fn push_success(output: &mut String, success: &Success) {
    push_start(output, success.kind.tag(), &success.path);
    if let Some(before) = success.before {
        push_stats(output, "before", before);
    }
    if let Some(after) = success.after {
        push_stats(output, "after", after);
    }
    push_uuid(output, "UUID", success.uuid);
    if let Some(undo_of) = success.undo_of {
        push_uuid(output, "UNDO_OF", undo_of);
    }
    push_end(output, success.kind.tag());
}
fn push_failure(output: &mut String, failure: &Failure) {
    if let Some(operation) = failure.operation.as_ref() {
        push_start(output, operation.0.tag(), &operation.1);
        push_reason(output, &failure.reason);
        push_end(output, operation.0.tag());
    } else if let Some(uuid) = failure.undo_uuid.as_ref() {
        output.push_str("<UNDO>\n<UUID>\n");
        output.push_str(uuid);
        output.push_str("\n</UUID>\n");
        push_reason(output, &failure.reason);
        output.push_str("</UNDO>\n");
    } else {
        push_reason(output, &failure.reason);
    }
}
fn push_start(output: &mut String, tag: &str, path: &Path) {
    if let Err(error) = writeln!(output, "<{tag}>\n{}", path.display()) {
        panic!("writing to String failed: {error}");
    }
}
fn push_end(output: &mut String, tag: &str) {
    if let Err(error) = writeln!(output, "</{tag}>") {
        panic!("writing to String failed: {error}");
    }
}
fn push_uuid(output: &mut String, tag: &str, uuid: OperationId) {
    if let Err(error) = writeln!(output, "<{tag}>\n{uuid}\n</{tag}>") {
        panic!("writing to String failed: {error}");
    }
}
fn push_stats(output: &mut String, label: &str, stats: FileStats) {
    if let Err(error) = writeln!(
        output,
        "{label}: {} lines, {} chars",
        stats.line_count, stats.character_count
    ) {
        panic!("writing to String failed: {error}");
    }
}
fn push_reason(output: &mut String, reason: &str) {
    output.push_str("<REASON>\n");
    output.push_str(reason);
    if !reason.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("</REASON>\n");
}
