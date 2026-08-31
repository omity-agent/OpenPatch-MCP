use super::{
    super::{
        OperationService,
        model::{Mutation, OperationId, OperationKind},
        result::{Failure, OperationOutput, Success},
    },
    files, reversal,
};
pub(crate) fn execute(service: &OperationService, uuid_texts: &[String]) -> OperationOutput {
    if uuid_texts.is_empty() {
        return OperationOutput::failed(String::from("uuids must not be empty"));
    }
    let mut output = OperationOutput::default();
    let mut targets = Vec::with_capacity(uuid_texts.len());
    for uuid_text in uuid_texts {
        match OperationId::parse(uuid_text) {
            Ok(uuid) => targets.push((uuid_text.clone(), uuid)),
            Err(error) => output.push_failure(Failure::undo(uuid_text.clone(), error.to_string())),
        }
    }
    let target_ids = targets
        .iter()
        .map(|target_entry| target_entry.1)
        .collect::<Vec<_>>();
    let order = match super::super::store::for_undo(&service.history, &target_ids) {
        Ok(order) => order,
        Err(error) => {
            let reason = error.to_string();
            for (uuid_text, _) in targets {
                output.push_failure(Failure::undo(uuid_text, reason.clone()));
            }
            return output;
        }
    };
    for index in order {
        let Some((uuid_text, target)) = targets.get(index).cloned() else {
            output.push_failure(Failure::global(String::from(
                "undo target ordering returned an invalid index",
            )));
            return output;
        };
        match commit(service, target) {
            Ok((mutation, uuid)) => output.push_success(success(&mutation, uuid, target)),
            Err(error) => output.push_failure(Failure::undo(uuid_text, error.to_string())),
        }
    }
    output
}
fn commit(
    service: &OperationService,
    target: OperationId,
) -> anyhow::Result<(Mutation, OperationId)> {
    let mut connection = service.history.lock_connection()?;
    let mut staged = None;
    let mut commit_attempted = false;
    let result = connection.immediate_transaction(|transaction| {
        service.history.ensure_available(transaction, target)?;
        let stored = super::super::store::HistoryStore::load(transaction, target)?;
        let mutation = reversal::plan(&stored)?;
        let uuid = service
            .history
            .consume_and_insert(transaction, target, &mutation)?;
        service.history.prune(transaction)?;
        files::apply(&mutation)?;
        staged = Some(mutation);
        commit_attempted = true;
        Ok(uuid)
    });
    match result {
        Ok(uuid) => {
            drop(connection);
            staged.map(|mutation| (mutation, uuid)).ok_or_else(|| {
                anyhow::anyhow!("undo transaction committed without a file mutation")
            })
        }
        Err(error) => {
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
            let Some(mutation) = staged else {
                return Err(history_error);
            };
            match files::roll_back(&mutation) {
                Ok(()) => Err(history_error),
                Err(rollback_error) => Err(anyhow::anyhow!(
                    "Failed to commit undo history: {history_error}; failed to roll back files: {rollback_error}"
                )),
            }
        }
    }
}
fn success(mutation: &Mutation, uuid: OperationId, target: OperationId) -> Success {
    let (before, after) = super::logical_stats(mutation);
    Success::new(
        mutation.kind,
        uuid,
        Some(target),
        mutation.display_path.clone(),
        (mutation.kind != OperationKind::Add)
            .then_some(before)
            .flatten(),
        (mutation.kind != OperationKind::Delete)
            .then_some(after)
            .flatten(),
    )
}
