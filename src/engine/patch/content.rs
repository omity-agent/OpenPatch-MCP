use crate::{parser::UpdateChunk, seek_sequence};
use replacements::{Replacement, apply_replacements};
use smallvec::SmallVec;
mod diagnostic;
mod matching;
mod replacements;
pub(crate) struct DerivedContents {
    pub(crate) before_contents: DerivedText,
    pub(crate) contents: DerivedText,
    pub(crate) applied_chunks: usize,
    pub(crate) errors: Vec<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DerivedText {
    Original,
    Modified(String),
}
pub(crate) fn derive_new_contents(
    original_contents: &str,
    chunks: &[UpdateChunk],
) -> DerivedContents {
    let mut original_lines = Vec::with_capacity(crate::text::line_count(original_contents));
    original_lines.extend(original_contents.lines());
    let plan = compute_replacements(&original_lines, chunks);
    let before_contents = render_contents(
        original_contents,
        &original_lines,
        &plan.reverse_replacements,
    );
    let contents = render_contents(
        original_contents,
        &original_lines,
        &plan.forward_replacements,
    );
    DerivedContents {
        before_contents,
        contents,
        applied_chunks: plan.applied_chunks,
        errors: plan.errors,
    }
}
struct ReplacementPlan<'chunk> {
    forward_replacements: SmallVec<[Replacement<'chunk>; 4]>,
    reverse_replacements: SmallVec<[Replacement<'chunk>; 4]>,
    applied_chunks: usize,
    errors: Vec<String>,
}
fn compute_replacements<'chunk>(
    original_lines: &[&str],
    chunks: &'chunk [UpdateChunk],
) -> ReplacementPlan<'chunk> {
    let search_index = seek_sequence::LineSearchIndex::new(original_lines);
    let mut forward_replacements = SmallVec::with_capacity(chunks.len());
    let mut reverse_replacements = SmallVec::with_capacity(chunks.len());
    let mut errors = Vec::new();
    let mut applied_chunks = 0;
    let mut line_index = 0;
    for (index, chunk) in chunks.iter().enumerate() {
        let Some(remaining_chunks) = chunks.get(index..) else {
            panic!("chunk index must be in bounds");
        };
        match matching::plan_chunk(
            original_lines,
            &search_index,
            chunk,
            remaining_chunks,
            line_index,
        ) {
            Ok(chunk_plan) => {
                if let Some(replacement) = chunk_plan.forward {
                    forward_replacements.push(replacement);
                }
                if let Some(replacement) = chunk_plan.reverse {
                    reverse_replacements.push(replacement);
                }
                line_index = chunk_plan.next_line_index;
                applied_chunks += 1;
            }
            Err(error) => errors.push(error.to_string()),
        }
    }
    ReplacementPlan {
        forward_replacements,
        reverse_replacements,
        applied_chunks,
        errors,
    }
}
fn render_contents(
    original_contents: &str,
    lines: &[&str],
    replacements: &[Replacement<'_>],
) -> DerivedText {
    if replacements.is_empty() {
        return DerivedText::Original;
    }
    DerivedText::Modified(apply_replacements(original_contents, lines, replacements))
}
