use super::{
    super::{
        OperationService,
        model::{Mutation, OperationId, OperationKind},
        result::{Failure, OperationOutput, Success},
    },
    durable, reversal,
    workspace::PlannedMutation,
};
pub(crate) fn execute(service: &OperationService, id_texts: &[String]) -> OperationOutput {
    if id_texts.is_empty() {
        return OperationOutput::failed(String::from("ids must not be empty"));
    }
    let guard = match service.begin() {
        Ok(guard) => guard,
        Err(error) => return OperationOutput::failed(format!("{error:#}")),
    };
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
    let order = match service.history.for_undo(&target_ids) {
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
            Err(error) => output.push_failure(Failure::undo(id_text, format!("{error:#}"))),
        }
    }
    drop(guard);
    output
}
fn commit(
    service: &OperationService,
    target: OperationId,
) -> anyhow::Result<(Mutation, OperationId)> {
    durable::resume(&service.history)?;
    let stored = service.history.load(target)?;
    let mutation = reversal::plan(&stored)?;
    let planned = PlannedMutation {
        observed: mutation
            .changes
            .iter()
            .map(|change| change.before.clone())
            .collect(),
        mutation,
    };
    let ids = durable::commit(
        &service.history,
        core::slice::from_ref(&planned),
        Some(target),
    )?;
    let id = ids
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("undo committed without an operation ID"))?;
    Ok((planned.mutation, id))
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
