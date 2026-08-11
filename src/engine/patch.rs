pub(crate) mod content;
use crate::{
    operation::{
        files,
        model::{FileContents, FileState, Mutation, OperationKind},
    },
    parser::{FileHunk, UpdateChunk},
    text::LineEndings,
};
use content::{DerivedText, derive_new_contents};
use std::path::PathBuf;
pub(crate) struct PlannedHunk {
    pub(crate) mutation: Option<Mutation>,
    pub(crate) observed: Vec<FileState>,
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
    let observed = files::snapshot(&path, "Failed to inspect file before adding")?;
    let proposed_after = FileState::present(contents);
    let (before, after) = if observed == proposed_after {
        (FileState::Missing, observed.clone())
    } else {
        (observed.clone(), proposed_after)
    };
    Ok(PlannedHunk {
        mutation: Some(Mutation::single(OperationKind::Add, path, before, after)),
        observed: vec![observed],
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
            before.clone(),
            FileState::Missing,
        )),
        observed: vec![before],
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
    let source_observed = files::snapshot(&path, "Failed to read file to update")?;
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
            observed: Vec::new(),
            chunk_errors,
        });
    }
    let (mutation, observed) = match move_path {
        None => (
            Mutation::single(OperationKind::Edit, path, source_before, destination_after),
            vec![source_observed],
        ),
        Some(destination) if destination == path => (
            Mutation::single(OperationKind::Edit, path, source_before, destination_after),
            vec![source_observed],
        ),
        Some(destination) => {
            let destination_observed =
                files::snapshot(&destination, "Failed to inspect move destination")?;
            (
                Mutation::moved(
                    path,
                    destination,
                    source_before,
                    destination_observed.clone(),
                    destination_after,
                ),
                vec![destination_observed, source_observed],
            )
        }
    };
    Ok(PlannedHunk {
        mutation: Some(mutation),
        observed,
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
