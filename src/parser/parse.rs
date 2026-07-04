use crate::{
    parser::{
        ADD_FILE_MARKER, CHANGE_CONTEXT_MARKER, DELETE_FILE_MARKER, EMPTY_CHANGE_CONTEXT_MARKER,
        EOF_MARKER, FileHunk, MOVE_TO_MARKER, ParseFailure, UPDATE_FILE_MARKER, UpdateChunk,
        boundary,
    },
    path_expansion::expand_path,
};
use std::path::PathBuf;
pub fn parse_patch(patch: &str) -> Result<Vec<FileHunk>, ParseFailure> {
    boundary::parse_patch_lines(patch)
}
pub(crate) fn parse_hunks(lines: &[&str]) -> Result<Vec<FileHunk>, ParseFailure> {
    let mut hunks = Vec::with_capacity(lines.len().saturating_div(3));
    let mut index = 1;
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if let Some(path) = line.strip_prefix(ADD_FILE_MARKER) {
            let (hunk, next_index) = parse_add(lines, index, path)?;
            hunks.push(hunk);
            index = next_index;
        } else if let Some(path) = line.strip_prefix(DELETE_FILE_MARKER) {
            hunks.push(FileHunk::Delete {
                path: parse_path(path, index)?,
            });
            index += 1;
        } else if let Some(path) = line.strip_prefix(UPDATE_FILE_MARKER) {
            let (hunk, next_index) = parse_update(lines, index, path)?;
            hunks.push(hunk);
            index = next_index;
        } else {
            return Err(ParseFailure::hunk(
                index + 1,
                "expected file operation marker",
            ));
        }
    }
    Ok(hunks)
}
fn parse_add(
    lines: &[&str],
    marker_index: usize,
    path: &str,
) -> Result<(FileHunk, usize), ParseFailure> {
    let mut contents = String::new();
    let mut line_count = 0;
    let mut character_count = 0;
    let mut index = marker_index + 1;
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if is_file_marker(line) {
            break;
        }
        let Some(content) = line.strip_prefix('+') else {
            return Err(ParseFailure::hunk(
                index + 1,
                "add file lines must start with '+'",
            ));
        };
        contents.push_str(content);
        contents.push('\n');
        line_count += 1;
        character_count += line_character_count(content) + 1;
        index += 1;
    }
    Ok((
        FileHunk::Add {
            path: parse_path(path, marker_index)?,
            contents,
            line_count,
            character_count,
        },
        index,
    ))
}
fn parse_update(
    lines: &[&str],
    marker_index: usize,
    path: &str,
) -> Result<(FileHunk, usize), ParseFailure> {
    let mut index = marker_index + 1;
    let move_path = lines
        .get(index)
        .and_then(|line| line.strip_prefix(MOVE_TO_MARKER))
        .map(|move_path_text| parse_path(move_path_text, index))
        .transpose()?;
    if move_path.is_some() {
        index += 1;
    }
    let mut chunks = Vec::new();
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if is_file_marker(line) {
            break;
        }
        let change_context = parse_chunk_context(line, index)?;
        let (chunk, next_index) = parse_chunk(lines, index + 1, change_context)?;
        chunks.push(chunk);
        index = next_index;
    }
    if chunks.is_empty() && move_path.is_none() {
        return Err(ParseFailure::hunk(
            marker_index + 1,
            "update file hunk has no changes",
        ));
    }
    Ok((
        FileHunk::Update {
            path: parse_path(path, marker_index)?,
            move_path,
            chunks,
        },
        index,
    ))
}
fn parse_chunk_context(line: &str, index: usize) -> Result<Option<String>, ParseFailure> {
    if line == EMPTY_CHANGE_CONTEXT_MARKER {
        Ok(None)
    } else if let Some(context) = line.strip_prefix(CHANGE_CONTEXT_MARKER) {
        Ok(Some(context.to_owned()))
    } else {
        Err(ParseFailure::hunk(index + 1, "expected '@@' change marker"))
    }
}
fn parse_chunk(
    lines: &[&str],
    start_index: usize,
    change_context: Option<String>,
) -> Result<(UpdateChunk, usize), ParseFailure> {
    let mut old_lines = crate::parser::ChunkLines::new();
    let mut new_lines = crate::parser::ChunkLines::new();
    let mut is_end_of_file = false;
    let mut index = start_index;
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if is_file_marker(line) || is_chunk_marker(line) {
            break;
        }
        if line == EOF_MARKER {
            is_end_of_file = true;
        } else if let Some(content) = line.strip_prefix(' ') {
            old_lines.push(content.to_owned());
            new_lines.push(content.to_owned());
        } else if let Some(content) = line.strip_prefix('-') {
            old_lines.push(content.to_owned());
        } else if let Some(content) = line.strip_prefix('+') {
            new_lines.push(content.to_owned());
        } else {
            return Err(ParseFailure::hunk(index + 1, "expected change line prefix"));
        }
        index += 1;
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
fn is_file_marker(line: &str) -> bool {
    line.starts_with(ADD_FILE_MARKER)
        || line.starts_with(DELETE_FILE_MARKER)
        || line.starts_with(UPDATE_FILE_MARKER)
}
fn is_chunk_marker(line: &str) -> bool {
    line == EMPTY_CHANGE_CONTEXT_MARKER || line.starts_with(CHANGE_CONTEXT_MARKER)
}
fn parse_path(path: &str, marker_index: usize) -> Result<PathBuf, ParseFailure> {
    expand_path(path).map_err(|error| ParseFailure::hunk(marker_index + 1, &error.to_string()))
}
fn line_character_count(line: &str) -> usize {
    if line.is_ascii() {
        line.len()
    } else {
        bytecount::num_chars(line.as_bytes())
    }
}
