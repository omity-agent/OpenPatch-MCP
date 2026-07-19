use super::{
    OperationService, files,
    model::{Mutation, OperationId, OperationKind, PathRole},
    output::{Failure, FileStats, OperationOutput, Success},
};
use crate::parser::FileHunk;
use rusqlite::TransactionBehavior;
pub(super) fn execute(service: &OperationService, patch: &str) -> OperationOutput {
    if patch.trim().is_empty() {
        return OperationOutput::failed(String::from("patch must not be empty"));
    }
    let hunks = match crate::parser::parse_patch(patch) {
        Ok(hunks) => hunks,
        Err(error) => return OperationOutput::failed(error.to_string()),
    };
    if hunks.is_empty() {
        return OperationOutput::failed(String::from("No files were modified."));
    }
    let mut output = OperationOutput::default();
    for hunk in hunks {
        execute_hunk(service, hunk, &mut output);
    }
    output
}
fn execute_hunk(service: &OperationService, hunk: FileHunk, output: &mut OperationOutput) {
    let failure_context = crate::patch::hunk_context(&hunk);
    let result = commit_hunk(service, hunk, output);
    match result {
        Ok(Some((mutation, uuid))) => output.push_success(success(&mutation, uuid)),
        Ok(None) => {}
        Err(error) => output.push_failure(Failure::file(
            failure_context.0,
            failure_context.1,
            error.to_string(),
        )),
    }
}
fn commit_hunk(
    service: &OperationService,
    hunk: FileHunk,
    output: &mut OperationOutput,
) -> anyhow::Result<Option<(Mutation, OperationId)>> {
    let mut connection = service.history.connection()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let planned = crate::patch::plan_hunk(hunk)?;
    let failure_path = planned
        .mutation
        .as_ref()
        .map(|mutation| mutation.display_path.clone());
    for reason in planned.chunk_errors {
        let path = failure_path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("<unknown>"));
        output.push_failure(Failure::file(OperationKind::Edit, path, reason));
    }
    let Some(mutation) = planned.mutation else {
        return Ok(None);
    };
    let uuid = service.history.insert(&transaction, &mutation, None)?;
    service.history.prune(&transaction)?;
    files::apply(&mutation)?;
    if let Err(error) = transaction.commit() {
        let rollback = files::roll_back(&mutation);
        return match rollback {
            Ok(()) => Err(error.into()),
            Err(rollback_error) => Err(anyhow::anyhow!(
                "Failed to commit operation history: {error}; failed to roll back files: {rollback_error}"
            )),
        };
    }
    Ok(Some((mutation, uuid)))
}
fn success(mutation: &Mutation, uuid: OperationId) -> Success {
    let (before, after) = logical_stats(mutation);
    Success::new(
        mutation.kind,
        uuid,
        None,
        mutation.display_path.clone(),
        (mutation.kind != OperationKind::Add)
            .then_some(before)
            .flatten(),
        (mutation.kind != OperationKind::Delete)
            .then_some(after)
            .flatten(),
    )
}
pub(super) fn logical_stats(mutation: &Mutation) -> (Option<FileStats>, Option<FileStats>) {
    let before = mutation
        .change(PathRole::Source)
        .or_else(|_| mutation.change(PathRole::Single))
        .ok()
        .and_then(|change| change.before.contents())
        .map(|contents| FileStats::from_contents(contents));
    let after = mutation
        .change(PathRole::Destination)
        .or_else(|_| mutation.change(PathRole::Single))
        .ok()
        .and_then(|change| change.after.contents())
        .map(|contents| FileStats::from_contents(contents));
    (before, after)
}
