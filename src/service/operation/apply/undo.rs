use super::{
    super::{
        OperationService,
        model::{Mutation, OperationId, OperationKind},
        result::{Failure, OperationOutput, Success},
    },
    files, reversal,
};
pub(crate) fn execute(service: &OperationService, id_texts: &[String]) -> OperationOutput {
    if id_texts.is_empty() {
        return OperationOutput::failed(String::from("ids must not be empty"));
    }
    let mut output = OperationOutput::default();
    let mut targets = Vec::with_capacity(id_texts.len());
    for id_text in id_texts {
        match OperationId::parse(id_text) {
            Ok(id) => targets.push((id_text.clone(), id)),
            Err(error) => output.push_failure(Failure::undo(id_text.clone(), error.to_string())),
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
            for (id_text, _) in targets {
                output.push_failure(Failure::undo(id_text, reason.clone()));
            }
            return output;
        }
    };
    for index in order {
        let Some((id_text, target)) = targets.get(index).cloned() else {
            output.push_failure(Failure::global(String::from(
                "undo target ordering returned an invalid index",
            )));
            return output;
        };
        match commit(service, target) {
            Ok((mutation, id)) => output.push_success(success(&mutation, id, target)),
            Err(error) => output.push_failure(Failure::undo(id_text, error.to_string())),
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
        let id = service
            .history
            .consume_and_insert(transaction, target, &mutation)?;
        service.history.prune(transaction)?;
        files::apply(&mutation)?;
        staged = Some(mutation);
        commit_attempted = true;
        Ok(id)
    });
    match result {
        Ok(id) => {
            drop(connection);
            staged.map(|mutation| (mutation, id)).ok_or_else(|| {
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
fn success(mutation: &Mutation, id: OperationId, target: OperationId) -> Success {
    let (before, after) = super::logical_stats(mutation);
    Success::new(
        mutation.kind,
        id,
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
