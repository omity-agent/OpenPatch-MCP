use crate::{
    operation::model::{FileState, Mutation, OperationKind},
    parser::FileHunk,
    text::LineEndings,
};
use smallvec::{SmallVec, smallvec};
use std::path::{Path, PathBuf};
pub(crate) struct PlannedHunk {
    pub(crate) mutations: SmallVec<[Mutation; 2]>,
    pub(crate) chunk_errors: Vec<String>,
}
pub(crate) fn plan_hunk(
    hunk: FileHunk,
    snapshot: &mut impl FnMut(&Path, &str) -> anyhow::Result<FileState>,
) -> anyhow::Result<PlannedHunk> {
    match hunk {
        FileHunk::Add {
            path,
            contents,
            line_count: _,
            character_count: _,
        } => plan_add(path, contents, snapshot),
        FileHunk::Delete { path } => plan_delete(path, snapshot),
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => super::updating::plan(path, move_path, &chunks, snapshot),
    }
}
fn plan_add(
    path: PathBuf,
    contents: String,
    snapshot: &mut impl FnMut(&Path, &str) -> anyhow::Result<FileState>,
) -> anyhow::Result<PlannedHunk> {
    let observed = snapshot(&path, "Failed to inspect file before adding")?;
    let proposed_after = FileState::present(LineEndings::normalize_owned(contents));
    let (before, after) = if observed == proposed_after {
        (FileState::Missing, observed)
    } else {
        (observed, proposed_after)
    };
    Ok(PlannedHunk {
        mutations: smallvec![Mutation::single(OperationKind::Add, path, before, after)],
        chunk_errors: Vec::new(),
    })
}
fn plan_delete(
    path: PathBuf,
    snapshot: &mut impl FnMut(&Path, &str) -> anyhow::Result<FileState>,
) -> anyhow::Result<PlannedHunk> {
    let before = snapshot(&path, "Failed to read file to delete")?;
    if before == FileState::Missing {
        anyhow::bail!("Failed to delete file: file does not exist");
    }
    Ok(PlannedHunk {
        mutations: smallvec![Mutation::single(
            OperationKind::Delete,
            path,
            before,
            FileState::Missing,
        )],
        chunk_errors: Vec::new(),
    })
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the hunk remains borrowed while its path is cloned for diagnostics"
)]
pub(crate) fn hunk_context(hunk: &FileHunk) -> (OperationKind, PathBuf) {
    match hunk {
        FileHunk::Add {
            path,
            contents: _,
            line_count: _,
            character_count: _,
        } => (OperationKind::Add, path.clone()),
        FileHunk::Delete { path } => (OperationKind::Delete, path.clone()),
        FileHunk::Update {
            path,
            move_path,
            chunks: _,
        } => match move_path {
            Some(destination) if destination != path => (OperationKind::Move, destination.clone()),
            _ => (OperationKind::Edit, path.clone()),
        },
    }
}
