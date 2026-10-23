use super::{
    super::model::{FileState, Mutation, OperationKind},
    workspace::{PlannedMutation, VirtualFile, Workspace},
};
use rustc_hash::FxHashSet;
use smallvec::smallvec;
struct OrderedMutation {
    planned: PlannedMutation,
    order: usize,
}
impl Workspace {
    pub(super) fn finish(mut self) -> Vec<PlannedMutation> {
        self.flush();
        self.completed
    }
    pub(super) fn flush(&mut self) {
        let mut claimed = FxHashSet::default();
        let mut ordered = self.moves(&mut claimed);
        for (path, file) in &self.files {
            if claimed.contains(path) {
                continue;
            }
            let Some(before) = effective_before(file).cloned() else {
                continue;
            };
            if before == file.current {
                continue;
            }
            ordered.push(OrderedMutation {
                planned: PlannedMutation {
                    mutation: Mutation::single(
                        transition_kind(&before, &file.current),
                        path.clone(),
                        before,
                        file.current.clone(),
                    ),
                    observed: smallvec![file.observed.clone()],
                },
                order: file.first_order,
            });
        }
        ordered.sort_by_key(|entry| entry.order);
        self.completed
            .extend(ordered.into_iter().map(|entry| entry.planned));
        for (path, file) in &mut self.files {
            file.observed = file.current.clone();
            file.origin = matches!(file.current, FileState::Present(_)).then(|| path.clone());
            file.logical_before = None;
        }
    }
    fn moves(&self, claimed: &mut FxHashSet<std::path::PathBuf>) -> Vec<OrderedMutation> {
        let mut ordered = Vec::new();
        for (destination_path, destination) in &self.files {
            let Some(origin_path) = destination.origin.as_ref() else {
                continue;
            };
            if origin_path == destination_path
                || claimed.contains(origin_path)
                || claimed.contains(destination_path)
            {
                continue;
            }
            let Some(source) = self.files.get(origin_path) else {
                continue;
            };
            let Some(source_before) = effective_before(source) else {
                continue;
            };
            let Some(destination_before) = effective_before(destination) else {
                continue;
            };
            if !matches!(source_before, FileState::Present(_))
                || source.current != FileState::Missing
                || !matches!(destination.current, FileState::Present(_))
                || destination_before == &destination.current
            {
                continue;
            }
            ordered.push(OrderedMutation {
                planned: PlannedMutation {
                    mutation: Mutation::moved(
                        origin_path.clone(),
                        destination_path.clone(),
                        source_before.clone(),
                        destination_before.clone(),
                    ),
                    observed: smallvec![destination.observed.clone(), source.observed.clone()],
                },
                order: source.first_order.min(destination.first_order),
            });
            claimed.insert(origin_path.clone());
            claimed.insert(destination_path.clone());
        }
        ordered
    }
}
fn effective_before(file: &VirtualFile) -> Option<&FileState> {
    let logical_before = file.logical_before.as_ref()?;
    if logical_before == &file.current && file.observed != file.current {
        Some(&file.observed)
    } else {
        Some(logical_before)
    }
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the operation kind is derived directly from two borrowed states"
)]
const fn transition_kind(before: &FileState, after: &FileState) -> OperationKind {
    match (before, after) {
        (FileState::Missing, FileState::Present(_)) => OperationKind::Add,
        (FileState::Present(_), FileState::Missing) => OperationKind::Delete,
        _ => OperationKind::Edit,
    }
}
