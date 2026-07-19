use crate::{parser::UpdateChunk, seek_sequence};
use replacements::{Replacement, apply_replacements};
use smallvec::SmallVec;
mod diagnostic;
mod matching;
mod replacements;
pub(crate) struct DerivedContents {
    pub(crate) before_contents: String,
    pub(crate) contents: String,
    pub(crate) applied_chunks: usize,
    pub(crate) errors: Vec<String>,
}
pub(crate) fn derive_new_contents(
    original_contents: &str,
    chunks: &[UpdateChunk],
) -> DerivedContents {
    let line_analysis = split_lines(original_contents);
    let plan = compute_replacements(&line_analysis.lines, chunks);
    let before_contents = render_contents(
        original_contents,
        &line_analysis,
        &plan.reverse_replacements,
    );
    let contents = render_contents(
        original_contents,
        &line_analysis,
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
    let mut search_index = seek_sequence::LineSearchIndex::new(original_lines);
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
            &mut search_index,
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
    line_analysis: &LineAnalysis<'_>,
    replacements: &[Replacement<'_>],
) -> String {
    if replacements.is_empty() {
        return original_contents.to_owned();
    }
    apply_replacements(
        original_contents,
        &line_analysis.lines,
        &line_analysis.offsets,
        replacements,
    )
}
struct LineAnalysis<'content> {
    lines: Vec<&'content str>,
    offsets: Vec<usize>,
}
fn split_lines(contents: &str) -> LineAnalysis<'_> {
    let line_capacity = crate::text::line_count(contents);
    let mut lines = Vec::with_capacity(line_capacity);
    let mut offsets = Vec::with_capacity(line_capacity);
    let mut start = 0;
    for index in memchr::memchr_iter(b'\n', contents.as_bytes()) {
        if let Some(line) = contents.get(start..index) {
            lines.push(line);
            offsets.push(start);
        }
        start = index + 1;
    }
    if start < contents.len()
        && let Some(line) = contents.get(start..)
    {
        lines.push(line);
        offsets.push(start);
    }
    LineAnalysis { lines, offsets }
}
#[cfg(test)]
mod tests;
