use crate::{parser::UpdateChunk, patch::summary::FileStats, seek_sequence};
use replacements::{Replacement, apply_replacements};
use smallvec::SmallVec;
mod replacements;
pub(crate) struct DerivedContents {
    pub(crate) contents: String,
    pub(crate) before: FileStats,
    pub(crate) applied_chunks: usize,
    pub(crate) errors: Vec<String>,
}
pub(crate) fn derive_new_contents(
    original_contents: &str,
    chunks: &[UpdateChunk],
) -> DerivedContents {
    let line_analysis = split_lines(original_contents);
    let plan = compute_replacements(&line_analysis.lines, chunks);
    let contents = if plan.replacements.is_empty() {
        String::new()
    } else {
        apply_replacements(
            original_contents,
            &line_analysis.lines,
            &line_analysis.offsets,
            &plan.replacements,
        )
    };
    DerivedContents {
        contents,
        before: line_analysis.stats,
        applied_chunks: plan.applied_chunks,
        errors: plan.errors,
    }
}
struct ReplacementPlan<'chunk> {
    replacements: SmallVec<[Replacement<'chunk>; 4]>,
    applied_chunks: usize,
    errors: Vec<String>,
}
fn compute_replacements<'chunk>(
    original_lines: &[&str],
    chunks: &'chunk [UpdateChunk],
) -> ReplacementPlan<'chunk> {
    let mut search_index = seek_sequence::LineSearchIndex::new(original_lines);
    let mut replacements = SmallVec::with_capacity(chunks.len());
    let mut errors = Vec::new();
    let mut applied_chunks = 0;
    let mut line_index = 0;
    for chunk in chunks {
        if let Some(context_line) = chunk.change_context.as_ref() {
            match seek_context(&mut search_index, context_line, line_index) {
                Ok(index) => line_index = index,
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            }
        }
        match make_replacement(original_lines, &mut search_index, chunk, line_index) {
            Ok((replacement, next_line_index)) => {
                replacements.push(replacement);
                line_index = next_line_index;
                applied_chunks += 1;
            }
            Err(error) => errors.push(error.to_string()),
        }
    }
    ReplacementPlan {
        replacements,
        applied_chunks,
        errors,
    }
}
fn seek_context(
    search_index: &mut seek_sequence::LineSearchIndex<'_, '_>,
    context_line: &String,
    line_index: usize,
) -> anyhow::Result<usize> {
    if let Some(sequence_match) =
        search_index.seek(core::slice::from_ref(context_line), line_index, false)
    {
        Ok(sequence_match.start + sequence_match.length)
    } else {
        anyhow::bail!("Failed to find context '{context_line}'");
    }
}
fn make_replacement<'chunk>(
    original_lines: &[&str],
    search_index: &mut seek_sequence::LineSearchIndex<'_, '_>,
    chunk: &'chunk UpdateChunk,
    line_index: usize,
) -> anyhow::Result<(Replacement<'chunk>, usize)> {
    if chunk.old_lines.is_empty() {
        let insertion_index = if original_lines.last().is_some_and(|line| line.is_empty()) {
            original_lines.len() - 1
        } else {
            original_lines.len()
        };
        return Ok((
            (insertion_index, 0, chunk.new_lines.as_slice()),
            insertion_index,
        ));
    }
    let mut pattern = chunk.old_lines.as_slice();
    let mut new_slice = chunk.new_lines.as_slice();
    let mut found = search_index.seek(pattern, line_index, chunk.is_end_of_file);
    if found.is_none() && pattern.last().is_some_and(String::is_empty) {
        if let Some((_, prefix)) = pattern.split_last() {
            pattern = prefix;
        }
        if new_slice.last().is_some_and(String::is_empty)
            && let Some((_, prefix)) = new_slice.split_last()
        {
            new_slice = prefix;
        }
        found = search_index.seek(pattern, line_index, chunk.is_end_of_file);
    }
    if let Some(sequence_match) = found {
        Ok((
            (sequence_match.start, sequence_match.length, new_slice),
            sequence_match.start + sequence_match.length,
        ))
    } else {
        anyhow::bail!(
            "Failed to find expected lines:\n{}",
            chunk.old_lines.join("\n")
        );
    }
}
struct LineAnalysis<'content> {
    lines: Vec<&'content str>,
    offsets: Vec<usize>,
    stats: FileStats,
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
    let stats = FileStats::from_counts(lines.len(), crate::text::character_count(contents));
    LineAnalysis {
        lines,
        offsets,
        stats,
    }
}
#[cfg(test)]
mod tests;
