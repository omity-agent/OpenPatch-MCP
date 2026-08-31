use super::{Failure, FileStats, OperationOutput, Success};
use core::fmt::Write as _;
pub(super) fn operations(result: &OperationOutput) -> String {
    let mut output = String::with_capacity((result.successes.len() + result.failures.len()) * 192);
    if !result.successes.is_empty() {
        output.push_str("<SUCCEEDED>\n");
        for success in &result.successes {
            push_success(&mut output, success);
        }
        output.push_str("</SUCCEEDED>");
    }
    if !result.failures.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("<FAILED>\n");
        for failure in &result.failures {
            push_failure(&mut output, failure);
        }
        output.push_str("</FAILED>");
    }
    output
}
fn push_success(output: &mut String, success: &Success) {
    push_start(output, success.kind.tag(), &success.path);
    if let Some(before) = success.before {
        push_stats(output, "before", before);
    }
    if let Some(after) = success.after {
        push_stats(output, "after", after);
    }
    push_uuid(output, "UUID", &success.uuid);
    if let Some(undo_of) = success.undo_of.as_deref() {
        push_uuid(output, "UNDO_OF", undo_of);
    }
    push_end(output, success.kind.tag());
}
fn push_failure(output: &mut String, failure: &Failure) {
    if let Some(operation) = failure.operation.as_ref() {
        push_start(output, operation.kind.tag(), &operation.path);
        push_reason(output, &failure.reason);
        push_end(output, operation.kind.tag());
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
fn push_start(output: &mut String, tag: &str, path: &str) {
    if let Err(error) = writeln!(output, "<{tag}>\n{path}") {
        panic!("writing to String failed: {error}");
    }
}
fn push_end(output: &mut String, tag: &str) {
    if let Err(error) = writeln!(output, "</{tag}>") {
        panic!("writing to String failed: {error}");
    }
}
fn push_uuid(output: &mut String, tag: &str, uuid: &str) {
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
