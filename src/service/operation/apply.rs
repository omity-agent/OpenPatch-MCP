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
    files::apply_observed(&mutation, &planned.observed)?;
    if let Err(error) = transaction.commit() {
        let rollback = files::roll_back_observed(&mutation, &planned.observed);
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
#[cfg(test)]
mod tests {
    use super::OperationService;
    use std::fs;
    #[test]
    fn already_applied_update_can_be_undone_to_patch_before_contents() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.txt");
        fs::write(&target, "new\n").unwrap();
        let service = OperationService::open(&directory.path().join("history.sqlite3")).unwrap();
        let patch = format!(
            "*** Begin Patch\n*** Update File: {}\n@@\n-old from patch\n+new\n*** End Patch",
            target.display()
        );
        let applied = service.apply(&patch);
        assert!(applied.succeeded(), "{}", applied.render());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new\n");
        let rendered = applied.render();
        let (_, uuid_suffix) = rendered.split_once("<UUID>\n").unwrap();
        let (uuid, _) = uuid_suffix.split_once("\n</UUID>").unwrap();
        let undone = service.undo(&[uuid.to_owned()]);
        assert!(undone.succeeded(), "{}", undone.render());
        assert_eq!(fs::read_to_string(target).unwrap(), "old from patch\n");
    }
    #[test]
    fn already_applied_add_can_be_undone_to_a_missing_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.txt");
        fs::write(&target, "added\n").unwrap();
        let service = OperationService::open(&directory.path().join("history.sqlite3")).unwrap();
        let patch = format!(
            "*** Begin Patch\n*** Add File: {}\n+added\n*** End Patch",
            target.display()
        );
        let applied = service.apply(&patch);
        assert!(applied.succeeded(), "{}", applied.render());
        assert_eq!(fs::read_to_string(&target).unwrap(), "added\n");
        let rendered = applied.render();
        let (_, uuid_suffix) = rendered.split_once("<UUID>\n").unwrap();
        let (uuid, _) = uuid_suffix.split_once("\n</UUID>").unwrap();
        let undone = service.undo(&[uuid.to_owned()]);
        assert!(undone.succeeded(), "{}", undone.render());
        assert!(!target.exists());
        assert!(undone.render().contains("<DELETE>"));
    }
}
