use core::fmt::Write as _;
use smallvec::SmallVec;
use std::{path::Path, path::PathBuf};
type FileSuccesses = SmallVec<[FileSuccess; 1]>;
#[derive(Debug, Default)]
pub(crate) struct Summary {
    successes: FileSuccesses,
    failures: Vec<FileFailure>,
}
impl Summary {
    pub(crate) fn push_success(&mut self, success: FileSuccess) {
        self.successes.push(success);
    }
    pub(crate) fn push_failure(&mut self, failure: FileFailure) {
        self.failures.push(failure);
    }
    pub(crate) fn failed(reason: String) -> Self {
        Self {
            successes: FileSuccesses::new(),
            failures: vec![FileFailure::global(reason)],
        }
    }
    pub(crate) const fn succeeded(&self) -> bool {
        self.failures.is_empty()
    }
    pub(crate) fn render(&self) -> String {
        let mut output = String::with_capacity(self.render_capacity());
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
    fn render_capacity(&self) -> usize {
        (self.successes.len() + self.failures.len()) * 128
            + self
                .failures
                .iter()
                .map(|failure| failure.reason.len())
                .sum::<usize>()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSuccess {
    kind: OperationKind,
    path: PathBuf,
    before: Option<FileStats>,
    after: Option<FileStats>,
}
impl FileSuccess {
    pub(crate) const fn add(path: PathBuf, after: FileStats) -> Self {
        Self {
            kind: OperationKind::Add,
            path,
            before: None,
            after: Some(after),
        }
    }
    pub(crate) const fn edit(path: PathBuf, before: FileStats, after: FileStats) -> Self {
        Self {
            kind: OperationKind::Edit,
            path,
            before: Some(before),
            after: Some(after),
        }
    }
    pub(crate) const fn delete(path: PathBuf, before: FileStats) -> Self {
        Self {
            kind: OperationKind::Delete,
            path,
            before: Some(before),
            after: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationKind {
    Add,
    Edit,
    Delete,
}
impl OperationKind {
    const fn tag(self) -> &'static str {
        match self {
            Self::Add => "ADD",
            Self::Edit => "EDIT",
            Self::Delete => "DELETE",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileFailure {
    operation: Option<(OperationKind, PathBuf)>,
    reason: String,
}
impl FileFailure {
    pub(crate) const fn file(kind: OperationKind, path: PathBuf, reason: String) -> Self {
        Self {
            operation: Some((kind, path)),
            reason,
        }
    }
    const fn global(reason: String) -> Self {
        Self {
            operation: None,
            reason,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStats {
    line_count: usize,
    character_count: usize,
}
impl FileStats {
    pub(crate) const fn from_counts(line_count: usize, character_count: usize) -> Self {
        Self {
            line_count,
            character_count,
        }
    }
    pub(crate) fn from_contents(contents: &str) -> Self {
        Self {
            line_count: crate::text::line_count(contents),
            character_count: crate::text::character_count(contents),
        }
    }
}
fn push_success(output: &mut String, success: &FileSuccess) {
    push_operation_start(output, success.kind, &success.path);
    if let Some(before) = success.before {
        push_stat_line(output, "before", before);
    }
    if let Some(after) = success.after {
        push_stat_line(output, "after", after);
    }
    push_operation_end(output, success.kind);
}
fn push_failure(output: &mut String, failure: &FileFailure) {
    if let Some(operation) = failure.operation.as_ref() {
        push_operation_start(output, operation.0, &operation.1);
        push_reason(output, &failure.reason);
        push_operation_end(output, operation.0);
    } else {
        push_reason(output, &failure.reason);
    }
}
fn push_operation_start(output: &mut String, kind: OperationKind, path: &Path) {
    if let Err(error) = writeln!(output, "<{}>\n{}", kind.tag(), path.display()) {
        panic!("writing to String failed: {error}");
    }
}
fn push_operation_end(output: &mut String, kind: OperationKind) {
    if let Err(error) = writeln!(output, "</{}>", kind.tag()) {
        panic!("writing to String failed: {error}");
    }
}
fn push_stat_line(output: &mut String, label: &str, stats: FileStats) {
    let mut line_buffer = itoa::Buffer::new();
    let mut character_buffer = itoa::Buffer::new();
    if let Err(error) = writeln!(
        output,
        "{label}: {} lines, {} chars",
        line_buffer.format(stats.line_count),
        character_buffer.format(stats.character_count)
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
