use crate::{parser::UpdateChunk, seek_sequence};
use replacements::{Replacement, apply_replacements};
use std::path::Path;
mod replacements;
pub(crate) struct DerivedContents {
    pub(crate) contents: String,
    pub(crate) applied_chunks: usize,
    pub(crate) errors: Vec<String>,
}
pub(crate) fn derive_new_contents(
    path: &Path,
    original_contents: &str,
    chunks: &[UpdateChunk],
) -> DerivedContents {
    let mut original_lines: Vec<&str> = original_contents.split('\n').collect();
    if original_lines.last().is_some_and(|line| line.is_empty()) {
        original_lines.pop();
    }
    let plan = compute_replacements(&original_lines, path, chunks);
    DerivedContents {
        contents: apply_replacements(original_contents, &original_lines, &plan.replacements),
        applied_chunks: plan.applied_chunks,
        errors: plan.errors,
    }
}
struct ReplacementPlan {
    replacements: Vec<Replacement>,
    applied_chunks: usize,
    errors: Vec<String>,
}
fn compute_replacements(
    original_lines: &[&str],
    path: &Path,
    chunks: &[UpdateChunk],
) -> ReplacementPlan {
    let mut search_index = seek_sequence::LineSearchIndex::new(original_lines);
    let mut replacements = Vec::new();
    let mut errors = Vec::new();
    let mut applied_chunks = 0;
    let mut line_index = 0;
    for chunk in chunks {
        if let Some(context_line) = chunk.change_context.as_ref() {
            match seek_context(&mut search_index, path, context_line, line_index) {
                Ok(index) => line_index = index,
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            }
        }
        match make_replacement(original_lines, &mut search_index, path, chunk, line_index) {
            Ok((replacement, next_line_index)) => {
                replacements.push(replacement);
                line_index = next_line_index;
                applied_chunks += 1;
            }
            Err(error) => errors.push(error.to_string()),
        }
    }
    replacements.sort_by_key(|replacement| replacement.0);
    ReplacementPlan {
        replacements,
        applied_chunks,
        errors,
    }
}
fn seek_context(
    search_index: &mut seek_sequence::LineSearchIndex<'_, '_>,
    path: &Path,
    context_line: &String,
    line_index: usize,
) -> anyhow::Result<usize> {
    if let Some(sequence_match) =
        search_index.seek(core::slice::from_ref(context_line), line_index, false)
    {
        Ok(sequence_match.start + sequence_match.length)
    } else {
        anyhow::bail!(
            "Failed to find context '{context_line}' in {}",
            path.display()
        );
    }
}
fn make_replacement(
    original_lines: &[&str],
    search_index: &mut seek_sequence::LineSearchIndex<'_, '_>,
    path: &Path,
    chunk: &UpdateChunk,
    line_index: usize,
) -> anyhow::Result<(Replacement, usize)> {
    if chunk.old_lines.is_empty() {
        let insertion_index = if original_lines.last().is_some_and(|line| line.is_empty()) {
            original_lines.len() - 1
        } else {
            original_lines.len()
        };
        return Ok((
            (insertion_index, 0, chunk.new_lines.clone()),
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
            (
                sequence_match.start,
                sequence_match.length,
                new_slice.to_vec(),
            ),
            sequence_match.start + sequence_match.length,
        ))
    } else {
        anyhow::bail!(
            "Failed to find expected lines in {}:\n{}",
            path.display(),
            chunk.old_lines.join("\n")
        );
    }
}
#[cfg(test)]
mod tests;
