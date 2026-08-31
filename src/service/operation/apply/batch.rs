use super::{
    super::{
        OperationService,
        model::{FileStates, Mutation, OperationId},
        result::{Failure, OperationOutput},
    },
    files, success,
    workspace::{self, PlannedMutation},
};
use crate::parser::FileHunk;
use diesel::{Connection as _, sqlite::SqliteConnection};
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
    let hunk_contexts = hunks
        .iter()
        .map(crate::patch::hunk_context)
        .collect::<Vec<_>>();
    let mut connection = match service.history.lock_connection() {
        Ok(connection) => connection,
        Err(error) => {
            push_context_failures(&mut output, &hunk_contexts, &error.to_string());
            return output;
        }
    };
    let mut staged = Vec::new();
    let mut transaction_started = false;
    let mut commit_attempted = false;
    let result = connection.immediate_transaction(|transaction| {
        transaction_started = true;
        let plan = workspace::plan(hunks);
        for failure in plan.failures {
            output.push_failure(Failure::file(failure.kind, failure.path, failure.reason));
        }
        if plan.eliminated_all {
            output.push_failure(Failure::global(String::from(
                "No effective file operations remained after merging.",
            )));
        }
        let mut remaining = plan.mutations.into_iter();
        while let Some(planned) = remaining.next() {
            let kind = planned.mutation.kind;
            let path = planned.mutation.display_path.clone();
            match stage_mutation(service, transaction, planned) {
                Ok(applied) => staged.push(applied),
                Err(StageFailure::Mutation(error)) => {
                    output.push_failure(Failure::file(kind, path, error.to_string()));
                }
                Err(StageFailure::Fatal(error)) => {
                    output.push_failure(Failure::file(kind, path, error.to_string()));
                    let reason = format!("Operation batch aborted: {error}");
                    push_mutation_failures(&mut output, remaining.as_slice(), &reason);
                    return Err(error);
                }
            }
        }
        if !staged.is_empty() {
            service.history.prune(transaction)?;
        }
        commit_attempted = true;
        Ok(())
    });
    if let Err(error) = result {
        let history_error = if commit_attempted {
            match super::super::store::roll_back_transaction(&mut connection) {
                Ok(()) => error,
                Err(rollback_error) => anyhow::anyhow!(
                    "{error}; failed to roll back history transaction: {rollback_error}"
                ),
            }
        } else {
            error
        };
        drop(connection);
        if !transaction_started {
            push_context_failures(&mut output, &hunk_contexts, &history_error.to_string());
        }
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
    connection: &mut SqliteConnection,
    planned: PlannedMutation,
) -> Result<StagedMutation, StageFailure> {
    let mut files_applied = false;
    let transaction_result = connection.transaction(|savepoint| {
        let uuid = service.history.insert(savepoint, &planned.mutation, None)?;
        files::apply_observed(&planned.mutation, &planned.observed)?;
        files_applied = true;
        Ok(uuid)
    });
    let uuid = match transaction_result {
        Ok(uuid) => uuid,
        Err(error) if !files_applied => return Err(StageFailure::Mutation(error)),
        Err(error) => {
            let rollback = files::roll_back_observed(&planned.mutation, &planned.observed);
            let fatal = match rollback {
                Ok(()) => error,
                Err(rollback_error) => anyhow::anyhow!(
                    "Failed to release history savepoint: {error}; failed to roll back files: {rollback_error}"
                ),
            };
            return Err(StageFailure::Fatal(fatal));
        }
    };
    Ok(StagedMutation {
        mutation: planned.mutation,
        observed: planned.observed,
        uuid,
    })
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
fn push_context_failures(
    output: &mut OperationOutput,
    contexts: &[(crate::operation::model::OperationKind, std::path::PathBuf)],
    reason: &str,
) {
    for (kind, path) in contexts.iter().cloned() {
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
