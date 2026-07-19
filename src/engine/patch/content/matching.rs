use super::{diagnostic::match_failure_reason, replacements::Replacement};
use crate::{parser::UpdateChunk, seek_sequence::LineSearchIndex};
pub(super) struct ChunkPlan<'chunk> {
    pub(super) forward: Option<Replacement<'chunk>>,
    pub(super) reverse: Option<Replacement<'chunk>>,
    pub(super) next_line_index: usize,
}
pub(super) fn plan_chunk<'chunk>(
    original_lines: &[&str],
    search_index: &mut LineSearchIndex<'_, '_>,
    chunk: &'chunk UpdateChunk,
    remaining_chunks: &'chunk [UpdateChunk],
    line_index: usize,
) -> anyhow::Result<ChunkPlan<'chunk>> {
    let search_start = match chunk.change_context.as_ref() {
        Some(context) => seek_context(original_lines, search_index, context, line_index)?,
        None => line_index,
    };
    if chunk.old_lines.is_empty() {
        return plan_insertion(original_lines, chunk, remaining_chunks, search_start);
    }
    let old_lines = chunk.old_lines.as_slice();
    let new_lines = chunk.new_lines.as_slice();
    if let Some(sequence_match) =
        search_index.seek_exact(old_lines, search_start, chunk.is_end_of_file)
    {
        return Ok(forward_plan(sequence_match, new_lines));
    }
    if !new_lines.is_empty()
        && let Some(sequence_match) =
            search_index.seek_exact(new_lines, search_start, chunk.is_end_of_file)
    {
        return Ok(reverse_plan(sequence_match, old_lines));
    }
    let (fallback_old, fallback_new) = without_logical_trailing_empty(old_lines, new_lines);
    if fallback_old.len() != old_lines.len() {
        if let Some(sequence_match) =
            search_index.seek_exact(fallback_old, search_start, chunk.is_end_of_file)
        {
            return Ok(forward_plan(sequence_match, fallback_new));
        }
        if !fallback_new.is_empty()
            && let Some(sequence_match) =
                search_index.seek_exact(fallback_new, search_start, chunk.is_end_of_file)
        {
            return Ok(reverse_plan(sequence_match, fallback_old));
        }
    }
    if let Some(sequence_match) = search_index.seek(old_lines, search_start, chunk.is_end_of_file) {
        return Ok(forward_plan(sequence_match, new_lines));
    }
    if fallback_old.len() != old_lines.len()
        && let Some(sequence_match) =
            search_index.seek(fallback_old, search_start, chunk.is_end_of_file)
    {
        return Ok(forward_plan(sequence_match, fallback_new));
    }
    if fallback_new.is_empty() {
        let insertion_index = if chunk.is_end_of_file {
            logical_end(original_lines)
        } else {
            search_start
        };
        return Ok(ChunkPlan {
            forward: None,
            reverse: Some((insertion_index, 0, fallback_old)),
            next_line_index: insertion_index,
        });
    }
    anyhow::bail!(match_failure_reason(
        "Failed to find expected lines",
        original_lines,
        search_index,
        fallback_old,
    ));
}
const fn forward_plan(
    sequence_match: crate::seek_sequence::SequenceMatch,
    new_lines: &[String],
) -> ChunkPlan<'_> {
    ChunkPlan {
        forward: Some((sequence_match.start, sequence_match.length, new_lines)),
        reverse: None,
        next_line_index: sequence_match.start + sequence_match.length,
    }
}
const fn reverse_plan(
    sequence_match: crate::seek_sequence::SequenceMatch,
    old_lines: &[String],
) -> ChunkPlan<'_> {
    ChunkPlan {
        forward: None,
        reverse: Some((sequence_match.start, sequence_match.length, old_lines)),
        next_line_index: sequence_match.start + sequence_match.length,
    }
}
fn seek_context(
    original_lines: &[&str],
    search_index: &mut LineSearchIndex<'_, '_>,
    context: &String,
    line_index: usize,
) -> anyhow::Result<usize> {
    let pattern = core::slice::from_ref(context);
    if let Some(sequence_match) = search_index.seek(pattern, line_index, false) {
        return Ok(sequence_match.start + sequence_match.length);
    }
    anyhow::bail!(match_failure_reason(
        "Failed to find context",
        original_lines,
        search_index,
        pattern,
    ));
}
fn without_logical_trailing_empty<'chunk>(
    old_lines: &'chunk [String],
    new_lines: &'chunk [String],
) -> (&'chunk [String], &'chunk [String]) {
    let Some((last_old, old_prefix)) = old_lines.split_last() else {
        return (old_lines, new_lines);
    };
    if !last_old.is_empty() {
        return (old_lines, new_lines);
    }
    let adjusted_new = new_lines
        .strip_suffix(&[String::new()])
        .unwrap_or(new_lines);
    (old_prefix, adjusted_new)
}
fn plan_insertion<'chunk>(
    original_lines: &[&str],
    chunk: &'chunk UpdateChunk,
    remaining_chunks: &'chunk [UpdateChunk],
    line_index: usize,
) -> anyhow::Result<ChunkPlan<'chunk>> {
    let insertion_index = logical_end(original_lines);
    let new_lines = chunk.new_lines.as_slice();
    let insertion_run = remaining_chunks
        .iter()
        .take_while(|candidate| candidate.old_lines.is_empty());
    let inserted_line_count = insertion_run
        .clone()
        .try_fold(0_usize, |count, candidate| {
            count
                .checked_add(candidate.new_lines.len())
                .ok_or_else(|| anyhow::anyhow!("inserted line count overflowed"))
        })?;
    let already_inserted = insertion_index
        .checked_sub(inserted_line_count)
        .filter(|start| *start >= line_index)
        .filter(|start| {
            original_lines
                .get(*start..insertion_index)
                .is_some_and(|actual| {
                    actual.iter().copied().eq(insertion_run
                        .clone()
                        .flat_map(|candidate| candidate.new_lines.iter().map(String::as_str)))
                })
        });
    Ok(already_inserted.map_or_else(
        || ChunkPlan {
            forward: Some((insertion_index, 0, new_lines)),
            reverse: None,
            next_line_index: insertion_index,
        },
        |start| ChunkPlan {
            forward: None,
            reverse: Some((start, new_lines.len(), chunk.old_lines.as_slice())),
            next_line_index: start + new_lines.len(),
        },
    ))
}
fn logical_end(lines: &[&str]) -> usize {
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.len() - 1
    } else {
        lines.len()
    }
}
