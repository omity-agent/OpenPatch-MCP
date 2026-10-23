use super::{
    super::model::{FileState, FileStates, Mutation, OperationKind, PathRole},
    files,
};
use crate::parser::FileHunk;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use std::path::{Path, PathBuf};
pub(super) struct PlannedMutation {
    pub(super) mutation: Mutation,
    pub(super) observed: FileStates,
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
pub(super) struct Workspace {
    pub(super) files: IndexMap<PathBuf, VirtualFile, FxBuildHasher>,
    pub(super) mutation_count: usize,
    pub(super) completed: Vec<PlannedMutation>,
    moving: Option<bool>,
}
pub(super) struct VirtualFile {
    pub(super) observed: FileState,
    pub(super) current: FileState,
    pub(super) origin: Option<PathBuf>,
    pub(super) logical_before: Option<FileState>,
    pub(super) first_order: usize,
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the hunk remains borrowed while its source path is cloned for chunk diagnostics"
)]
pub(super) fn plan(hunks: Vec<FileHunk>) -> BatchPlan {
    let mut workspace = Workspace::default();
    let mut failures = Vec::new();
    for hunk in hunks {
        let (kind, path) = crate::patch::hunk_context(&hunk);
        let chunk_path = match &hunk {
            FileHunk::Add {
                path: source,
                contents: _,
                line_count: _,
                character_count: _,
            }
            | FileHunk::Delete { path: source }
            | FileHunk::Update {
                path: source,
                move_path: _,
                chunks: _,
            } => source.clone(),
        };
        let planned = crate::patch::plan_hunk(hunk, &mut |snapshot_path, action| {
            workspace.snapshot(snapshot_path, action)
        });
        match planned {
            Ok(planned_hunk) => {
                for reason in planned_hunk.chunk_errors {
                    failures.push(PlanningFailure {
                        kind: OperationKind::Edit,
                        path: chunk_path.clone(),
                        reason,
                    });
                }
                for mutation in planned_hunk.mutations {
                    let mutation_kind = mutation.kind;
                    let mutation_path = mutation.display_path.clone();
                    if let Err(error) = workspace.record(mutation) {
                        failures.push(PlanningFailure {
                            kind: mutation_kind,
                            path: mutation_path,
                            reason: error.to_string(),
                        });
                        break;
                    }
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
        if let Some(file) = self.files.get(path) {
            return Ok(file.current.clone());
        }
        let observed = files::snapshot(path, action)?;
        let origin = matches!(observed, FileState::Present(_)).then(|| path.to_owned());
        self.files.insert(
            path.to_owned(),
            VirtualFile {
                observed: observed.clone(),
                current: observed.clone(),
                origin,
                logical_before: None,
                first_order: 0,
            },
        );
        Ok(observed)
    }
    fn record(&mut self, mutation: Mutation) -> anyhow::Result<()> {
        let moving = mutation.kind == OperationKind::Move;
        if self.moving.is_some_and(|previous| previous != moving) {
            self.flush();
        }
        self.moving = Some(moving);
        let order = self.mutation_count;
        self.mutation_count = self
            .mutation_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("too many patch operations"))?;
        let moved = if mutation.changes.len() == 2 {
            let source = mutation.change(PathRole::Source)?;
            let destination = mutation.change(PathRole::Destination)?;
            let origin = self
                .files
                .get(&source.path)
                .ok_or_else(|| anyhow::anyhow!("move source has no virtual file state"))?
                .origin
                .clone();
            Some((source.path.clone(), destination.path.clone(), origin))
        } else {
            None
        };
        for change in mutation.changes {
            let file = self.file_mut(&change.path)?;
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
        self.files
            .get_mut(path)
            .ok_or_else(|| anyhow::anyhow!("planned path has no virtual file state"))
    }
}
