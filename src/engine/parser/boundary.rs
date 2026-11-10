use crate::parser::{BEGIN_PATCH_MARKER, END_PATCH_MARKER, FileHunk, ParseFailure};
pub(crate) fn parse_patch_lines(patch: &str) -> Result<Vec<FileHunk>, ParseFailure> {
    let trimmed_patch = patch.trim();
    let mut lines = Vec::with_capacity(crate::text::line_count(trimmed_patch));
    lines.extend(trimmed_patch.lines());
    let patch_lines = check_lenient(&lines)?;
    crate::parser::hunks::parse_hunks(patch_lines)
}
fn check_lenient<'line>(
    original_lines: &'line [&'line str],
) -> Result<&'line [&'line str], ParseFailure> {
    match check_strict(original_lines) {
        Ok(lines) => Ok(lines),
        Err(original_error) => match *original_lines {
            [first, .., last]
                if matches!(first, "<<EOF" | "<<'EOF'" | "<<\"EOF\"")
                    && last.ends_with("EOF")
                    && original_lines.len() >= 4 =>
            {
                check_strict(&original_lines[1..original_lines.len() - 1])
            }
            _ => Err(original_error),
        },
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
