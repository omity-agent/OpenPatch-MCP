pub(crate) mod content;
use crate::{
    operation::{
        files,
        model::{FileState, Mutation, OperationKind},
    },
    parser::{FileHunk, UpdateChunk},
};
use content::derive_new_contents;
use std::path::PathBuf;
pub(crate) struct PlannedHunk {
    pub(crate) mutation: Option<Mutation>,
    pub(crate) chunk_errors: Vec<String>,
}
pub(crate) fn plan_hunk(hunk: FileHunk) -> anyhow::Result<PlannedHunk> {
    match hunk {
        FileHunk::Add { path, contents, .. } => plan_add(path, contents),
        FileHunk::Delete { path } => plan_delete(path),
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => plan_update(path, move_path, &chunks),
    }
}
fn plan_add(path: PathBuf, contents: String) -> anyhow::Result<PlannedHunk> {
    let before = files::snapshot(&path, "Failed to inspect file before adding")?;
    Ok(PlannedHunk {
        mutation: Some(Mutation::single(
            OperationKind::Add,
            path,
            before,
            FileState::Present(contents),
        )),
        chunk_errors: Vec::new(),
    })
}
fn plan_delete(path: PathBuf) -> anyhow::Result<PlannedHunk> {
    let before = files::snapshot(&path, "Failed to read file to delete")?;
    if before == FileState::Missing {
        anyhow::bail!("Failed to delete file: file does not exist");
    }
    Ok(PlannedHunk {
        mutation: Some(Mutation::single(
            OperationKind::Delete,
            path,
            before,
            FileState::Missing,
        )),
        chunk_errors: Vec::new(),
    })
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the source contents remain borrowed while deriving the updated contents"
)]
fn plan_update(
    path: PathBuf,
    move_path: Option<PathBuf>,
    chunks: &[UpdateChunk],
) -> anyhow::Result<PlannedHunk> {
    let source_before = files::snapshot(&path, "Failed to read file to update")?;
    let FileState::Present(original) = &source_before else {
        anyhow::bail!("Failed to read file to update: file does not exist");
    };
    let (after_contents, chunk_errors, applied_chunks) = if chunks.is_empty() {
        (original.clone(), Vec::new(), 1)
    } else {
        let derived = derive_new_contents(original, chunks);
        (derived.contents, derived.errors, derived.applied_chunks)
    };
    if applied_chunks == 0 {
        return Ok(PlannedHunk {
            mutation: None,
            chunk_errors,
        });
    }
    let destination_after = FileState::Present(after_contents);
    let mutation = match move_path {
        None => Mutation::single(OperationKind::Edit, path, source_before, destination_after),
        Some(destination) if destination == path => {
            Mutation::single(OperationKind::Edit, path, source_before, destination_after)
        }
        Some(destination) => {
            let destination_before =
                files::snapshot(&destination, "Failed to inspect move destination")?;
            Mutation::moved(
                path,
                destination,
                source_before,
                destination_before,
                destination_after,
            )
        }
    };
    Ok(PlannedHunk {
        mutation: Some(mutation),
        chunk_errors,
    })
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the hunk is borrowed because it is consumed after its failure context is captured"
)]
pub(crate) fn hunk_context(hunk: &FileHunk) -> (OperationKind, PathBuf) {
    match hunk {
        FileHunk::Add { path, .. } => (OperationKind::Add, path.clone()),
        FileHunk::Delete { path } => (OperationKind::Delete, path.clone()),
        FileHunk::Update {
            path, move_path, ..
        } => (
            OperationKind::Edit,
            move_path.clone().unwrap_or_else(|| path.clone()),
        ),
    }
}
