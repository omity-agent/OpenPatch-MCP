use super::super::{
    files,
    model::{FileState, Mutation, OperationKind, PathRole},
};
use crate::parser::FileHunk;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
mod finalize;
pub(super) struct PlannedMutation {
    pub(super) mutation: Mutation,
    pub(super) observed: Vec<FileState>,
}
pub(super) struct PlanningFailure {
    pub(super) kind: OperationKind,
    pub(super) path: PathBuf,
    pub(super) reason: String,
}
pub(super) struct BatchPlan {
    pub(super) mutations: Vec<PlannedMutation>,
    pub(super) failures: Vec<PlanningFailure>,
    pub(super) eliminated_all: bool,
}
#[derive(Default)]
struct Workspace {
    files: Vec<VirtualFile>,
    indices: FxHashMap<PathBuf, usize>,
    mutation_count: usize,
}
struct VirtualFile {
    path: PathBuf,
    observed: FileState,
    current: FileState,
    origin: Option<PathBuf>,
    logical_before: Option<FileState>,
    first_order: usize,
}
pub(super) fn plan(hunks: Vec<FileHunk>) -> BatchPlan {
    let mut workspace = Workspace::default();
    let mut failures = Vec::new();
    for hunk in hunks {
        let (kind, path) = crate::patch::hunk_context(&hunk);
        let planned = crate::patch::plan_hunk(hunk, &mut |snapshot_path, action| {
            workspace.snapshot(snapshot_path, action)
        });
        match planned {
            Ok(planned_hunk) => {
                for reason in planned_hunk.chunk_errors {
                    failures.push(PlanningFailure {
                        kind: OperationKind::Edit,
                        path: path.clone(),
                        reason,
                    });
                }
                if let Some(mutation) = planned_hunk.mutation
                    && let Err(error) = workspace.record(mutation)
                {
                    failures.push(PlanningFailure {
                        kind,
                        path,
                        reason: error.to_string(),
                    });
                }
            }
            Err(error) => failures.push(PlanningFailure {
                kind,
                path,
                reason: error.to_string(),
            }),
        }
    }
    let mutation_count = workspace.mutation_count;
    let mutations = workspace.finish();
    BatchPlan {
        eliminated_all: mutation_count != 0 && mutations.is_empty(),
        mutations,
        failures,
    }
}
impl Workspace {
    fn snapshot(&mut self, path: &Path, action: &str) -> anyhow::Result<FileState> {
        if let Some(index) = self.indices.get(path).copied() {
            return self
                .files
                .get(index)
                .map(|file| file.current.clone())
                .ok_or_else(|| anyhow::anyhow!("virtual file index is out of bounds"));
        }
        let observed = files::snapshot(path, action)?;
        let index = self.files.len();
        let origin = matches!(observed, FileState::Present(_)).then(|| path.to_owned());
        self.files.push(VirtualFile {
            path: path.to_owned(),
            observed: observed.clone(),
            current: observed.clone(),
            origin,
            logical_before: None,
            first_order: 0,
        });
        self.indices.insert(path.to_owned(), index);
        Ok(observed)
    }
    fn record(&mut self, mutation: Mutation) -> anyhow::Result<()> {
        let order = self.mutation_count;
        self.mutation_count = self
            .mutation_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("too many patch operations"))?;
        let moved = if mutation.changes.len() == 2 {
            let source = mutation.change(PathRole::Source)?;
            let destination = mutation.change(PathRole::Destination)?;
            let source_index = self
                .indices
                .get(&source.path)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("move source has no virtual file state"))?;
            let origin = self
                .files
                .get(source_index)
                .ok_or_else(|| anyhow::anyhow!("virtual file index is out of bounds"))?
                .origin
                .clone();
            Some((source.path.clone(), destination.path.clone(), origin))
        } else {
            None
        };
        for change in mutation.changes {
            let index = self
                .indices
                .get(&change.path)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("planned path has no virtual file state"))?;
            let file = self
                .files
                .get_mut(index)
                .ok_or_else(|| anyhow::anyhow!("virtual file index is out of bounds"))?;
            if file.logical_before.is_none() {
                file.logical_before = Some(change.before);
                file.first_order = order;
            }
            file.current = change.after;
            if moved.is_none() && file.current == FileState::Missing {
                file.origin = None;
            }
        }
        if let Some((source, destination, origin)) = moved {
            self.file_mut(&source)?.origin = None;
            self.file_mut(&destination)?.origin = origin;
        }
        Ok(())
    }
    fn file_mut(&mut self, path: &Path) -> anyhow::Result<&mut VirtualFile> {
        let index = self
            .indices
            .get(path)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("planned path has no virtual file state"))?;
        self.files
            .get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("virtual file index is out of bounds"))
    }
}
