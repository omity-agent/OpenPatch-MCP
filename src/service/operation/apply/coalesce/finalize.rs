use super::{FileState, Mutation, OperationKind, PlannedMutation, Workspace};
use rustc_hash::FxHashSet;
use smallvec::smallvec;
struct OrderedMutation {
    planned: PlannedMutation,
    order: usize,
}
impl Workspace {
    pub(super) fn finish(self) -> Vec<PlannedMutation> {
        let mut claimed = FxHashSet::default();
        let mut ordered = self.moves(&mut claimed);
        for file in self.files {
            if claimed.contains(&file.path) {
                continue;
            }
            let Some(before) = effective_before(&file).cloned() else {
                continue;
            };
            if before == file.current {
                continue;
            }
            ordered.push(OrderedMutation {
                planned: PlannedMutation {
                    mutation: Mutation::single(
                        transition_kind(&before, &file.current),
                        file.path,
                        before,
                        file.current,
                    ),
                    observed: smallvec![file.observed],
                },
                order: file.first_order,
            });
        }
        ordered.sort_by_key(|entry| entry.order);
        ordered.into_iter().map(|entry| entry.planned).collect()
    }
    fn moves(&self, claimed: &mut FxHashSet<std::path::PathBuf>) -> Vec<OrderedMutation> {
        let mut ordered = Vec::new();
        for destination in &self.files {
            let Some(origin_path) = destination.origin.as_ref() else {
                continue;
            };
            if origin_path == &destination.path
                || claimed.contains(origin_path)
                || claimed.contains(&destination.path)
            {
                continue;
            }
            let Some(source) = self.file(origin_path) else {
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
                        source.path.clone(),
                        destination.path.clone(),
                        source_before.clone(),
                        destination_before.clone(),
                        destination.current.clone(),
                    ),
                    observed: smallvec![destination.observed.clone(), source.observed.clone()],
                },
                order: source.first_order.min(destination.first_order),
            });
            claimed.insert(source.path.clone());
            claimed.insert(destination.path.clone());
        }
        ordered
    }
    fn file(&self, path: &std::path::Path) -> Option<&super::VirtualFile> {
        let index = self.indices.get(path)?;
        self.files.get(*index)
    }
}
fn effective_before(file: &super::VirtualFile) -> Option<&FileState> {
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
