use super::content::{DerivedText, derive_new_contents};
use crate::{
    operation::model::{FileContents, FileState, Mutation, OperationKind},
    parser::{FileHunk, UpdateChunk},
    text::LineEndings,
};
use std::path::{Path, PathBuf};
pub(crate) struct PlannedHunk {
    pub(crate) mutation: Option<Mutation>,
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
        } => plan_update(path, move_path, &chunks, snapshot),
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
        mutation: Some(Mutation::single(OperationKind::Add, path, before, after)),
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
    snapshot: &mut impl FnMut(&Path, &str) -> anyhow::Result<FileState>,
) -> anyhow::Result<PlannedHunk> {
    let source_observed = snapshot(&path, "Failed to read file to update")?;
    let FileState::Present(original) = &source_observed else {
        anyhow::bail!("Failed to read file to update: file does not exist");
    };
    let (source_before, destination_after, chunk_errors, applied_chunks) = if chunks.is_empty() {
        (
            FileState::share(original),
            FileState::share(original),
            Vec::new(),
            1,
        )
    } else {
        let line_endings = LineEndings::detect(original);
        let normalized_original = LineEndings::normalize(original);
        let derived = derive_new_contents(&normalized_original, chunks);
        (
            state_from_derived(
                derived.before_contents,
                original,
                line_endings,
                false,
                &normalized_original,
            ),
            state_from_derived(
                derived.contents,
                original,
                line_endings,
                true,
                &normalized_original,
            ),
            derived.errors,
            derived.applied_chunks,
        )
    };
    if applied_chunks == 0 {
        return Ok(PlannedHunk {
            mutation: None,
            chunk_errors,
        });
    }
    let mutation = match move_path {
        None => Mutation::single(OperationKind::Edit, path, source_before, destination_after),
        Some(destination) if destination == path => {
            Mutation::single(OperationKind::Edit, path, source_before, destination_after)
        }
        Some(destination) => {
            let destination_observed =
                snapshot(&destination, "Failed to inspect move destination")?;
            Mutation::moved(
                path,
                destination,
                source_before,
                destination_observed,
                destination_after,
            )
        }
    };
    Ok(PlannedHunk {
        mutation: Some(mutation),
        chunk_errors,
    })
}
fn state_from_derived(
    derived: DerivedText,
    original: &FileContents,
    line_endings: LineEndings,
    normalize_mixed_original: bool,
    normalized_original: &str,
) -> FileState {
    match derived {
        DerivedText::Original if !normalize_mixed_original || !line_endings.is_mixed() => {
            FileState::share(original)
        }
        DerivedText::Original => FileState::present(normalized_original.to_owned()),
        DerivedText::Modified(modified) => FileState::present(line_endings.render(modified)),
    }
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
        } => (
            OperationKind::Edit,
            move_path.clone().unwrap_or_else(|| path.clone()),
        ),
    }
}
