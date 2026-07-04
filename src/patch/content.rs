use crate::{parser::UpdateChunk, seek_sequence};
use std::path::Path;
type Replacement = (usize, usize, Vec<String>);
pub(crate) fn derive_new_contents(
    path: &Path,
    original_contents: &str,
    chunks: &[UpdateChunk],
) -> anyhow::Result<String> {
    let mut original_lines: Vec<String> =
        original_contents.split('\n').map(str::to_owned).collect();
    if original_lines.last().is_some_and(String::is_empty) {
        original_lines.pop();
    }
    let replacements = compute_replacements(&original_lines, path, chunks)?;
    let mut new_lines = apply_replacements(original_lines, &replacements);
    if !new_lines.last().is_some_and(String::is_empty) {
        new_lines.push(String::new());
    }
    Ok(new_lines.join("\n"))
}
fn compute_replacements(
    original_lines: &[String],
    path: &Path,
    chunks: &[UpdateChunk],
) -> anyhow::Result<Vec<Replacement>> {
    let mut replacements = Vec::new();
    let mut line_index = 0;
    for chunk in chunks {
        if let Some(context_line) = chunk.change_context.as_ref() {
            line_index = seek_context(original_lines, path, context_line, line_index)?;
        }
        add_replacement(
            original_lines,
            path,
            chunk,
            &mut replacements,
            &mut line_index,
        )?;
    }
    replacements.sort_by_key(|replacement| replacement.0);
    Ok(replacements)
}
fn seek_context(
    original_lines: &[String],
    path: &Path,
    context_line: &String,
    line_index: usize,
) -> anyhow::Result<usize> {
    if let Some(index) = seek_sequence::seek_sequence(
        original_lines,
        core::slice::from_ref(context_line),
        line_index,
        false,
    ) {
        Ok(index + 1)
    } else {
        anyhow::bail!(
            "Failed to find context '{context_line}' in {}",
            path.display()
        );
    }
}
fn add_replacement(
    original_lines: &[String],
    path: &Path,
    chunk: &UpdateChunk,
    replacements: &mut Vec<Replacement>,
    line_index: &mut usize,
) -> anyhow::Result<()> {
    if chunk.old_lines.is_empty() {
        let insertion_index = if original_lines.last().is_some_and(String::is_empty) {
            original_lines.len() - 1
        } else {
            original_lines.len()
        };
        replacements.push((insertion_index, 0, chunk.new_lines.clone()));
        return Ok(());
    }
    let mut pattern = chunk.old_lines.as_slice();
    let mut new_slice = chunk.new_lines.as_slice();
    let mut found =
        seek_sequence::seek_sequence(original_lines, pattern, *line_index, chunk.is_end_of_file);
    if found.is_none() && pattern.last().is_some_and(String::is_empty) {
        if let Some((_, prefix)) = pattern.split_last() {
            pattern = prefix;
        }
        if new_slice.last().is_some_and(String::is_empty)
            && let Some((_, prefix)) = new_slice.split_last()
        {
            new_slice = prefix;
        }
        found = seek_sequence::seek_sequence(
            original_lines,
            pattern,
            *line_index,
            chunk.is_end_of_file,
        );
    }
    if let Some(start_index) = found {
        replacements.push((start_index, pattern.len(), new_slice.to_vec()));
        *line_index = start_index + pattern.len();
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to find expected lines in {}:\n{}",
            path.display(),
            chunk.old_lines.join("\n")
        );
    }
}
fn apply_replacements(mut lines: Vec<String>, replacements: &[Replacement]) -> Vec<String> {
    for replacement in replacements.iter().rev() {
        let start_index = replacement.0;
        let old_length = replacement.1;
        for _ in 0..old_length {
            if start_index < lines.len() {
                lines.remove(start_index);
            }
        }
        for (offset, new_line) in replacement.2.iter().enumerate() {
            lines.insert(start_index + offset, new_line.clone());
        }
    }
    lines
}
#[cfg(test)]
mod tests {
    use super::derive_new_contents;
    use crate::parser::UpdateChunk;
    use std::path::Path;
    #[test]
    fn insertion_without_old_lines_precedes_logical_trailing_empty_line() {
        let chunk = UpdateChunk {
            change_context: None,
            old_lines: Vec::new(),
            new_lines: vec![String::from("b")],
            is_end_of_file: false,
        };
        let result = derive_new_contents(Path::new("target.txt"), "a\n\n", &[chunk]).unwrap();
        assert_eq!(result, "a\nb\n");
    }
}
