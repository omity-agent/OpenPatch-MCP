use super::{
    super::{
        OperationService, files,
        model::{FileState, Mutation, OperationId, OperationKind},
        output::{Failure, OperationOutput},
    },
    success,
};
use crate::parser::FileHunk;
use rusqlite::{Savepoint, Transaction, TransactionBehavior};
struct StagedHunk {
    mutation: Mutation,
    observed: Vec<FileState>,
    uuid: OperationId,
}
enum StageFailure {
    Hunk(anyhow::Error),
    Fatal(anyhow::Error),
}
impl From<anyhow::Error> for StageFailure {
    fn from(error: anyhow::Error) -> Self {
        Self::Hunk(error)
    }
}
pub(super) fn execute(service: &OperationService, hunks: Vec<FileHunk>) -> OperationOutput {
    let mut output = OperationOutput::default();
    let mut connection = match service.history.lock_connection() {
        Ok(connection) => connection,
        Err(error) => {
            push_hunk_failures(&mut output, &hunks, &error.to_string());
            return output;
        }
    };
    let mut transaction = match connection.transaction_with_behavior(TransactionBehavior::Immediate)
    {
        Ok(transaction) => transaction,
        Err(error) => {
            push_hunk_failures(&mut output, &hunks, &error.to_string());
            return output;
        }
    };
    let mut staged = Vec::new();
    let mut remaining = hunks.into_iter();
    while let Some(hunk) = remaining.next() {
        let failure_context = crate::patch::hunk_context(&hunk);
        match stage_hunk(service, &mut transaction, hunk, &mut output) {
            Ok(Some(applied)) => staged.push(applied),
            Ok(None) => {}
            Err(StageFailure::Hunk(error)) => output.push_failure(Failure::file(
                failure_context.0,
                failure_context.1,
                error.to_string(),
            )),
            Err(StageFailure::Fatal(error)) => {
                output.push_failure(Failure::file(
                    failure_context.0,
                    failure_context.1,
                    error.to_string(),
                ));
                drop(transaction);
                drop(connection);
                fail_staged(&mut output, &staged, &error);
                let reason = format!("Operation batch aborted: {error}");
                push_hunk_failures(&mut output, remaining.as_slice(), &reason);
                return output;
            }
        }
    }
    if staged.is_empty() {
        return output;
    }
    if let Err(error) = service.history.prune(&transaction) {
        drop(transaction);
        drop(connection);
        fail_staged(&mut output, &staged, &error);
        return output;
    }
    if let Err(error) = transaction.commit() {
        drop(connection);
        let history_error = error.into();
        fail_staged(&mut output, &staged, &history_error);
        return output;
    }
    drop(connection);
    for applied in staged {
        output.push_success(success(&applied.mutation, applied.uuid));
    }
    output
}
fn stage_hunk(
    service: &OperationService,
    transaction: &mut Transaction<'_>,
    hunk: FileHunk,
    output: &mut OperationOutput,
) -> Result<Option<StagedHunk>, StageFailure> {
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
    let observed = planned.observed;
    let savepoint = transaction
        .savepoint()
        .map_err(|error| StageFailure::Fatal(error.into()))?;
    let uuid = match service.history.insert(&savepoint, &mutation, None) {
        Ok(uuid) => uuid,
        Err(error) => return finish_failed_savepoint(savepoint, error),
    };
    if let Err(error) = files::apply_observed(&mutation, &observed) {
        return finish_failed_savepoint(savepoint, error);
    }
    if let Err(error) = savepoint.commit() {
        let rollback = files::roll_back_observed(&mutation, &observed);
        let fatal = match rollback {
            Ok(()) => error.into(),
            Err(rollback_error) => anyhow::anyhow!(
                "Failed to release history savepoint: {error}; failed to roll back files: {rollback_error}"
            ),
        };
        return Err(StageFailure::Fatal(fatal));
    }
    Ok(Some(StagedHunk {
        mutation,
        observed,
        uuid,
    }))
}
fn finish_failed_savepoint<T>(
    savepoint: Savepoint<'_>,
    error: anyhow::Error,
) -> Result<T, StageFailure> {
    match savepoint.finish() {
        Ok(()) => Err(StageFailure::Hunk(error)),
        Err(rollback_error) => Err(StageFailure::Fatal(anyhow::anyhow!(
            "{error}; failed to roll back history savepoint: {rollback_error}"
        ))),
    }
}
fn fail_staged(output: &mut OperationOutput, staged: &[StagedHunk], error: &anyhow::Error) {
    let mut rollback_errors = Vec::new();
    for applied in staged.iter().rev() {
        if let Err(rollback_error) = files::roll_back_observed(&applied.mutation, &applied.observed)
        {
            rollback_errors.push(format!(
                "{}: {rollback_error}",
                applied.mutation.display_path.display()
            ));
        }
    }
    let reason = if rollback_errors.is_empty() {
        format!("Failed to commit operation history: {error}")
    } else {
        format!(
            "Failed to commit operation history: {error}; failed to roll back files: {}",
            rollback_errors.join("; ")
        )
    };
    for applied in staged {
        output.push_failure(Failure::file(
            applied.mutation.kind,
            applied.mutation.display_path.clone(),
            reason.clone(),
        ));
    }
}
fn push_hunk_failures(output: &mut OperationOutput, hunks: &[FileHunk], reason: &str) {
    for hunk in hunks {
        let (kind, path) = crate::patch::hunk_context(hunk);
        output.push_failure(Failure::file(kind, path, reason.to_owned()));
    }
}
