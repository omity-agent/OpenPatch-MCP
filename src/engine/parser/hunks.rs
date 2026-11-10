use crate::{
    parser::{
        ADD_FILE_MARKER, DELETE_FILE_MARKER, FileHunk, MOVE_TO_MARKER, ParseFailure,
        UPDATE_FILE_MARKER, chunks::parse_chunk,
    },
    path_expansion::expand_path,
};
use std::path::PathBuf;
pub(crate) fn parse_hunks(lines: &[&str]) -> Result<Vec<FileHunk>, ParseFailure> {
    let mut hunks = Vec::with_capacity(lines.len().saturating_div(3));
    let mut index = 1;
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).map(|line| line.trim()) else {
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
    let mut index = marker_index + 1;
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if is_file_marker(line.trim()) {
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
        index += 1;
    }
    Ok((
        FileHunk::Add {
            path: parse_path(path, marker_index)?,
            contents,
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
    let move_path = parse_move_path(lines, index)?;
    if move_path.is_some() {
        index += 1;
    }
    let mut chunks = Vec::new();
    while index + 1 < lines.len() {
        let Some(line) = lines.get(index).copied() else {
            break;
        };
        if is_file_marker(line.trim_end()) {
            break;
        }
        let (chunk, next_index) = parse_chunk(lines, index, chunks.is_empty())?;
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
fn parse_move_path(lines: &[&str], index: usize) -> Result<Option<PathBuf>, ParseFailure> {
    let Some(line) = lines.get(index) else {
        return Ok(None);
    };
    let Some(path_text) = line.trim_end().strip_prefix(MOVE_TO_MARKER) else {
        return Ok(None);
    };
    Ok(Some(parse_path(path_text, index)?))
}
pub(super) fn is_file_marker(line: &str) -> bool {
    line.starts_with(ADD_FILE_MARKER)
        || line.starts_with(DELETE_FILE_MARKER)
        || line.starts_with(UPDATE_FILE_MARKER)
}
fn parse_path(path: &str, marker_index: usize) -> Result<PathBuf, ParseFailure> {
    let expanded = expand_path(path)
        .map_err(|error| ParseFailure::hunk(marker_index + 1, &error.to_string()))?;
    if !expanded.is_absolute() {
        return Err(ParseFailure::hunk(
            marker_index + 1,
            "patch paths must be absolute",
        ));
    }
    Ok(expanded)
}
