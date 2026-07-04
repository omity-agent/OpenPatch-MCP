use crate::parser::{BEGIN_PATCH_MARKER, END_PATCH_MARKER, FileHunk, ParseFailure};
pub(crate) fn parse_patch_lines(patch: &str) -> Result<Vec<FileHunk>, ParseFailure> {
    let trimmed_patch = patch.trim();
    let mut lines = Vec::with_capacity(line_capacity(trimmed_patch));
    lines.extend(trimmed_patch.lines());
    let patch_lines = check_lenient(&lines)?;
    crate::parser::parse::parse_hunks(patch_lines)
}
fn line_capacity(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        bytecount::count(text.as_bytes(), b'\n') + usize::from(!text.ends_with('\n'))
    }
}
fn check_lenient<'line>(
    original_lines: &'line [&'line str],
) -> Result<&'line [&'line str], ParseFailure> {
    match check_strict(original_lines) {
        Ok(lines) => Ok(lines),
        Err(original_error) => {
            let Some(first) = original_lines.first().copied() else {
                return Err(original_error);
            };
            let Some(last) = original_lines.last().copied() else {
                return Err(original_error);
            };
            if !is_heredoc_wrapper(first, last, original_lines.len()) {
                return Err(original_error);
            }
            let Some((_, tail)) = original_lines.split_first() else {
                return Err(original_error);
            };
            let Some((_, inner_lines)) = tail.split_last() else {
                return Err(original_error);
            };
            check_strict(inner_lines)
        }
    }
}
fn check_strict<'line>(lines: &'line [&'line str]) -> Result<&'line [&'line str], ParseFailure> {
    let first_trimmed = lines.first().map(|line| line.trim());
    let last_trimmed = lines.last().map(|line| line.trim());
    match (first_trimmed, last_trimmed) {
        (Some(first), Some(last)) if first == BEGIN_PATCH_MARKER && last == END_PATCH_MARKER => {
            Ok(lines)
        }
        (Some(first), _) if first != BEGIN_PATCH_MARKER => Err(ParseFailure::patch(
            "The first line of the patch must be '*** Begin Patch'",
        )),
        _ => Err(ParseFailure::patch(
            "The last line of the patch must be '*** End Patch'",
        )),
    }
}
fn is_heredoc_wrapper(first: &str, last: &str, line_count: usize) -> bool {
    matches!(first, "<<EOF" | "<<'EOF'" | "<<\"EOF\"") && last.ends_with("EOF") && line_count >= 4
}
