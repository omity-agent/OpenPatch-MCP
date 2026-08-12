use super::{
    super::{
        OperationService, files,
        model::{FileStates, Mutation, OperationId},
        output::{Failure, OperationOutput},
    },
    coalesce::{self, PlannedMutation},
    success,
};
use crate::parser::FileHunk;
use rusqlite::{Savepoint, Transaction, TransactionBehavior};
struct StagedMutation {
    mutation: Mutation,
    observed: FileStates,
    uuid: OperationId,
}
enum StageFailure {
    Mutation(anyhow::Error),
    Fatal(anyhow::Error),
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
    let plan = coalesce::plan(hunks);
    for failure in plan.failures {
        output.push_failure(Failure::file(failure.kind, failure.path, failure.reason));
    }
    if plan.eliminated_all {
        output.push_failure(Failure::global(String::from(
            "No effective file operations remained after merging.",
        )));
    }
    let mut staged = Vec::new();
    let mut remaining = plan.mutations.into_iter();
    while let Some(planned) = remaining.next() {
        let kind = planned.mutation.kind;
        let path = planned.mutation.display_path.clone();
        match stage_mutation(service, &mut transaction, planned) {
            Ok(applied) => staged.push(applied),
            Err(StageFailure::Mutation(error)) => {
                output.push_failure(Failure::file(kind, path, error.to_string()));
            }
            Err(StageFailure::Fatal(error)) => {
                output.push_failure(Failure::file(kind, path, error.to_string()));
                drop(transaction);
                drop(connection);
                fail_staged(&mut output, &staged, &error);
                let reason = format!("Operation batch aborted: {error}");
                push_mutation_failures(&mut output, remaining.as_slice(), &reason);
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
fn stage_mutation(
    service: &OperationService,
    transaction: &mut Transaction<'_>,
    planned: PlannedMutation,
) -> Result<StagedMutation, StageFailure> {
    let savepoint = transaction
        .savepoint()
        .map_err(|error| StageFailure::Fatal(error.into()))?;
    let uuid = match service.history.insert(&savepoint, &planned.mutation, None) {
        Ok(uuid) => uuid,
        Err(error) => return finish_failed_savepoint(savepoint, error),
    };
    if let Err(error) = files::apply_observed(&planned.mutation, &planned.observed) {
        return finish_failed_savepoint(savepoint, error);
    }
    if let Err(error) = savepoint.commit() {
        let rollback = files::roll_back_observed(&planned.mutation, &planned.observed);
        let fatal = match rollback {
            Ok(()) => error.into(),
            Err(rollback_error) => anyhow::anyhow!(
                "Failed to release history savepoint: {error}; failed to roll back files: {rollback_error}"
            ),
        };
        return Err(StageFailure::Fatal(fatal));
    }
    Ok(StagedMutation {
        mutation: planned.mutation,
        observed: planned.observed,
        uuid,
    })
}
fn finish_failed_savepoint<T>(
    savepoint: Savepoint<'_>,
    error: anyhow::Error,
) -> Result<T, StageFailure> {
    match savepoint.finish() {
        Ok(()) => Err(StageFailure::Mutation(error)),
        Err(rollback_error) => Err(StageFailure::Fatal(anyhow::anyhow!(
            "{error}; failed to roll back history savepoint: {rollback_error}"
        ))),
    }
}
fn fail_staged(output: &mut OperationOutput, staged: &[StagedMutation], error: &anyhow::Error) {
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
fn push_mutation_failures(
    output: &mut OperationOutput,
    mutations: &[PlannedMutation],
    reason: &str,
) {
    for planned in mutations {
        output.push_failure(Failure::file(
            planned.mutation.kind,
            planned.mutation.display_path.clone(),
            reason.to_owned(),
        ));
    }
}
