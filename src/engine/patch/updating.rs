use super::{
    content::{DerivedText, derive_new_contents},
    planning::PlannedHunk,
};
use crate::{
    operation::model::{FileContents, FileState, Mutation, OperationKind},
    parser::UpdateChunk,
    text::LineEndings,
};
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the source contents remain borrowed while deriving the updated contents"
)]
pub(super) fn plan(
    path: PathBuf,
    move_path: Option<PathBuf>,
    chunks: &[UpdateChunk],
    snapshot: &mut impl FnMut(&Path, &str) -> anyhow::Result<FileState>,
) -> anyhow::Result<PlannedHunk> {
    let source_observed = snapshot(&path, "Failed to read file to update")?;
    let FileState::Present(original) = &source_observed else {
        anyhow::bail!("Failed to read file to update: file does not exist");
    };
    let (source_before, source_after, chunk_errors, applied_chunks) = if chunks.is_empty() {
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
    let mut mutations = SmallVec::new();
    if applied_chunks != 0 {
        if let Some(destination) = move_path.filter(|destination| destination != &path) {
            let destination_before = snapshot(&destination, "Failed to inspect move destination")?;
            if source_before != source_after || source_observed != source_after {
                mutations.push(Mutation::single(
                    OperationKind::Edit,
                    path.clone(),
                    source_before,
                    source_after.clone(),
                ));
            }
            mutations.push(Mutation::moved(
                path,
                destination,
                source_after,
                destination_before,
            ));
        } else {
            mutations.push(Mutation::single(
                OperationKind::Edit,
                path,
                source_before,
                source_after,
            ));
        }
    }
    Ok(PlannedHunk {
        mutations,
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
