mod content;
mod fs_ops;
mod summary;
use crate::{
    parser::{FileHunk, UpdateChunk, parse_patch},
    patch::{content::derive_new_contents, fs_ops::FileWriter},
};
use std::path::PathBuf;
use summary::{FileFailure, FileStats, FileSuccess, OperationKind, Summary};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub output: String,
    pub succeeded: bool,
}
pub fn apply_patch_text(patch: &str) -> ApplyResult {
    let summary = if patch.trim().is_empty() {
        Summary::failed(String::from("patch must not be empty"))
    } else {
        apply_patch_text_inner(patch)
    };
    ApplyResult {
        succeeded: summary.succeeded(),
        output: summary.render(),
    }
}
fn apply_patch_text_inner(patch: &str) -> Summary {
    let hunks = match parse_patch(patch) {
        Ok(hunks) => hunks,
        Err(error) => return Summary::failed(error.to_string()),
    };
    if hunks.is_empty() {
        return Summary::failed(String::from("No files were modified."));
    }
    let mut summary = Summary::default();
    for hunk in hunks {
        apply_hunk(hunk, &mut summary);
    }
    summary
}
fn apply_hunk(hunk: FileHunk, summary: &mut Summary) {
    match hunk {
        FileHunk::Add {
            path,
            contents,
            line_count,
            character_count,
        } => apply_add(
            path,
            contents,
            FileStats::from_counts(line_count, character_count),
            summary,
        ),
        FileHunk::Delete { path } => apply_delete(path, summary),
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => apply_update(path, move_path, &chunks, summary),
    }
}
fn apply_add(path: PathBuf, contents: String, after: FileStats, summary: &mut Summary) {
    match FileWriter::write_with_parent_retry(&path, contents) {
        Ok(()) => summary.push_success(FileSuccess::add(path, after)),
        Err(error) => summary.push_failure(FileFailure::file(
            OperationKind::Add,
            path,
            error.to_string(),
        )),
    }
}
fn apply_delete(path: PathBuf, summary: &mut Summary) {
    let (target, original_contents) = match FileWriter::read_file_to_delete(&path) {
        Ok(file) => file,
        Err(error) => {
            summary.push_failure(FileFailure::file(
                OperationKind::Delete,
                path,
                error.to_string(),
            ));
            return;
        }
    };
    let before = FileStats::from_contents(&original_contents);
    match FileWriter::delete_resolved_file(&target) {
        Ok(()) => summary.push_success(FileSuccess::delete(path, before)),
        Err(error) => summary.push_failure(FileFailure::file(
            OperationKind::Delete,
            path,
            error.to_string(),
        )),
    }
}
fn apply_update(
    path: PathBuf,
    move_path: Option<PathBuf>,
    chunks: &[UpdateChunk],
    summary: &mut Summary,
) {
    let (source, original_contents) = match FileWriter::read_file_to_update(&path) {
        Ok(file) => file,
        Err(error) => {
            summary.push_failure(FileFailure::file(
                OperationKind::Edit,
                path,
                error.to_string(),
            ));
            return;
        }
    };
    if chunks.is_empty() {
        if let Some(destination) = move_path {
            apply_move(source, destination, original_contents, summary);
        }
        return;
    }
    let derived = derive_new_contents(&original_contents, chunks);
    for reason in derived.errors {
        summary.push_failure(FileFailure::file(OperationKind::Edit, path.clone(), reason));
    }
    if derived.applied_chunks == 0 {
        return;
    }
    let after = FileStats::from_contents(&derived.contents);
    if let Some(destination) = move_path {
        apply_moved_edit(
            source,
            destination,
            derived.contents,
            derived.before,
            after,
            summary,
        );
    } else {
        match FileWriter::write_resolved_file(&source, derived.contents) {
            Ok(()) => summary.push_success(FileSuccess::edit(path, derived.before, after)),
            Err(error) => summary.push_failure(FileFailure::file(
                OperationKind::Edit,
                path,
                error.to_string(),
            )),
        }
    }
}
fn apply_move(source: PathBuf, destination: PathBuf, contents: String, summary: &mut Summary) {
    let stats = FileStats::from_contents(&contents);
    apply_moved_edit(source, destination, contents, stats, stats, summary);
}
fn apply_moved_edit(
    source: PathBuf,
    destination: PathBuf,
    contents: String,
    before: FileStats,
    after: FileStats,
    summary: &mut Summary,
) {
    if let Err(error) = FileWriter::write_with_parent_retry(&destination, contents) {
        summary.push_failure(FileFailure::file(
            OperationKind::Edit,
            destination,
            error.to_string(),
        ));
        return;
    }
    summary.push_success(FileSuccess::edit(destination, before, after));
    if let Err(error) = FileWriter::delete_resolved_original(&source) {
        summary.push_failure(FileFailure::file(
            OperationKind::Edit,
            source,
            error.to_string(),
        ));
    }
}
#[cfg(test)]
mod tests;
