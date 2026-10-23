use super::{files, workspace::PlannedMutation};
use crate::operation::{
    model::OperationId,
    store::{HistoryStore, Transition},
};
use anyhow::Context as _;
use indexmap::{IndexMap, map::Entry};
use rustc_hash::FxBuildHasher;
pub(super) fn commit(
    history: &HistoryStore,
    planned: &[PlannedMutation],
    undo_of: Option<OperationId>,
) -> anyhow::Result<Vec<OperationId>> {
    let transitions = collapse(planned)?;
    files::verify(&transitions)?;
    let mutations = planned
        .iter()
        .map(|entry| entry.mutation.clone())
        .collect::<Vec<_>>();
    let ids = history.prepare(&mutations, &transitions, undo_of)?;
    let pending_ids = || {
        ids.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    files::publish(&transitions).with_context(|| {
        format!(
            "durable operations [{}] are pending forward recovery",
            pending_ids()
        )
    })?;
    history.finish().with_context(|| {
        format!(
            "durable operations [{}] are pending forward recovery",
            pending_ids()
        )
    })
}
pub(crate) fn resume(history: &HistoryStore) -> anyhow::Result<()> {
    let Some(transitions) = history.recovery()? else {
        return Ok(());
    };
    eprintln!("warning: completing an interrupted operation batch");
    files :: publish (& transitions) . context ("forward recovery blocked; durable intent is retained; restore conflicting files to their recorded state before retrying" ,) ? ;
    for id in history.finish()? {
        eprintln!("recovered operation: {id}");
    }
    Ok(())
}
pub(super) fn collapse(planned: &[PlannedMutation]) -> anyhow::Result<Vec<Transition>> {
    let mut transitions: IndexMap<&std::path::Path, Transition, FxBuildHasher> =
        IndexMap::default();
    for planned_mutation in planned {
        anyhow::ensure!(
            planned_mutation.observed.len() == planned_mutation.mutation.changes.len(),
            "file operation state count does not match its path count"
        );
        for (change, observed) in planned_mutation
            .mutation
            .changes
            .iter()
            .zip(&planned_mutation.observed)
        {
            match transitions.entry(change.path.as_path()) {
                Entry::Occupied(mut entry) => {
                    let transition = entry.get_mut();
                    anyhow::ensure!(
                        transition.after == *observed,
                        "discontinuous file operation plan: {}",
                        change.path.display()
                    );
                    transition.after = change.after.clone();
                }
                Entry::Vacant(entry) => {
                    entry.insert(Transition {
                        path: change.path.clone(),
                        before: observed.clone(),
                        after: change.after.clone(),
                    });
                }
            }
        }
    }
    transitions.sort_by_key(|_, transition| transition.after.contents().is_none());
    Ok(transitions.into_values().collect())
}
