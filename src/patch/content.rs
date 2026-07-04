use crate::{parser::UpdateChunk, seek_sequence};
use std::path::Path;
type Replacement = (usize, usize, Vec<String>);
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
    let mut original_lines: Vec<String> =
        original_contents.split('\n').map(str::to_owned).collect();
    if original_lines.last().is_some_and(String::is_empty) {
        original_lines.pop();
    }
    let plan = compute_replacements(&original_lines, path, chunks);
    let mut new_lines = apply_replacements(original_lines, &plan.replacements);
    if !new_lines.last().is_some_and(String::is_empty) {
        new_lines.push(String::new());
    }
    DerivedContents {
        contents: new_lines.join("\n"),
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
    original_lines: &[String],
    path: &Path,
    chunks: &[UpdateChunk],
) -> ReplacementPlan {
    let search_index = seek_sequence::LineSearchIndex::new(original_lines);
    let mut replacements = Vec::new();
    let mut errors = Vec::new();
    let mut applied_chunks = 0;
    let mut line_index = 0;
    for chunk in chunks {
        if let Some(context_line) = chunk.change_context.as_ref() {
            match seek_context(&search_index, path, context_line, line_index) {
                Ok(index) => line_index = index,
                Err(error) => {
                    errors.push(error.to_string());
                    continue;
                }
            }
        }
        match make_replacement(original_lines, &search_index, path, chunk, line_index) {
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
    search_index: &seek_sequence::LineSearchIndex,
    path: &Path,
    context_line: &String,
    line_index: usize,
) -> anyhow::Result<usize> {
    if let Some(index) = search_index.seek(core::slice::from_ref(context_line), line_index, false) {
        Ok(index + 1)
    } else {
        anyhow::bail!(
            "Failed to find context '{context_line}' in {}",
            path.display()
        );
    }
}
fn make_replacement(
    original_lines: &[String],
    search_index: &seek_sequence::LineSearchIndex,
    path: &Path,
    chunk: &UpdateChunk,
    line_index: usize,
) -> anyhow::Result<(Replacement, usize)> {
    if chunk.old_lines.is_empty() {
        let insertion_index = if original_lines.last().is_some_and(String::is_empty) {
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
    if let Some(start_index) = found {
        Ok((
            (start_index, pattern.len(), new_slice.to_vec()),
            start_index + pattern.len(),
        ))
    } else {
        anyhow::bail!(
            "Failed to find expected lines in {}:\n{}",
            path.display(),
            chunk.old_lines.join("\n")
        );
    }
}
fn apply_replacements(lines: Vec<String>, replacements: &[Replacement]) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    let mut source_lines = lines.into_iter();
    let mut source_index = 0;
    for replacement in replacements {
        let start_index = replacement.0;
        let old_length = replacement.1;
        assert!(
            source_index <= start_index,
            "replacement ranges must be ordered and in bounds"
        );
        while source_index < start_index {
            let Some(line) = source_lines.next() else {
                panic!("replacement ranges must be ordered and in bounds");
            };
            result.push(line);
            source_index += 1;
        }
        result.extend(replacement.2.iter().cloned());
        for _ in 0..old_length {
            if source_lines.next().is_none() {
                break;
            }
            source_index += 1;
        }
    }
    result.extend(source_lines);
    result
}
#[cfg(test)]
mod tests;
