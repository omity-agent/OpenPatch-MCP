use super::{
    CHANGE_CONTEXT_MARKER, ChunkLines, EMPTY_CHANGE_CONTEXT_MARKER, EOF_MARKER, ParseFailure,
    UpdateChunk, hunks::is_file_marker,
};
pub(super) fn parse_chunk(
    lines: &[&str],
    start_index: usize,
    first_chunk: bool,
) -> Result<(UpdateChunk, usize), ParseFailure> {
    let header = lines[start_index].trim_end();
    let (change_context, mut index) = if header == EMPTY_CHANGE_CONTEXT_MARKER {
        (None, start_index + 1)
    } else if let Some(context) = header.strip_prefix(CHANGE_CONTEXT_MARKER) {
        (Some(context.to_owned()), start_index + 1)
    } else if first_chunk {
        (None, start_index)
    } else {
        return Err(ParseFailure::hunk(
            start_index + 1,
            "expected '@@' change marker",
        ));
    };
    let mut old_lines = ChunkLines::new();
    let mut new_lines = ChunkLines::new();
    let mut is_end_of_file = false;
    while index + 1 < lines.len() {
        let line = lines[index];
        let marker = line.trim_end();
        if is_file_marker(marker)
            || marker == EMPTY_CHANGE_CONTEXT_MARKER
            || marker.starts_with(CHANGE_CONTEXT_MARKER)
        {
            break;
        }
        index += 1;
        if marker == EOF_MARKER {
            is_end_of_file = true;
            break;
        } else if line.is_empty() {
            old_lines.push(String::new());
            new_lines.push(String::new());
        } else if let Some(content) = line.strip_prefix(' ') {
            old_lines.push(content.to_owned());
            new_lines.push(content.to_owned());
        } else if let Some(content) = line.strip_prefix('-') {
            old_lines.push(content.to_owned());
        } else if let Some(content) = line.strip_prefix('+') {
            new_lines.push(content.to_owned());
        } else {
            return Err(ParseFailure::hunk(index, "expected change line prefix"));
        }
    }
    if old_lines.is_empty() && new_lines.is_empty() {
        return Err(ParseFailure::hunk(
            start_index + 1,
            "update chunk has no changes",
        ));
    }
    if is_end_of_file {
        while lines
            .get(index)
            .is_some_and(|line| line.trim_end().is_empty())
        {
            index += 1;
        }
    }
    Ok((
        UpdateChunk {
            change_context,
            old_lines,
            new_lines,
            is_end_of_file,
        },
        index,
    ))
}
