mod batch;
use super::{
    OperationService,
    model::{Mutation, OperationId, OperationKind, PathRole},
    output::{Failure, FileStats, OperationOutput, Success},
};
use crate::parser::{ChunkLines, FileHunk, UpdateChunk};
pub(super) fn execute(service: &OperationService, patch: &str) -> OperationOutput {
    if patch.trim().is_empty() {
        return OperationOutput::failed(String::from("patch must not be empty"));
    }
    let hunks = match crate::parser::parse_patch(patch) {
        Ok(hunks) => hunks,
        Err(error) => return OperationOutput::failed(error.to_string()),
    };
    if hunks.is_empty() {
        return OperationOutput::failed(String::from("No files were modified."));
    }
    batch::execute(service, hunks)
}
pub(super) fn execute_replacement(
    service: &OperationService,
    path: &str,
    old_string: &str,
    new_string: &str,
) -> OperationOutput {
    let display_path = std::path::PathBuf::from(path);
    let expanded_path = match expanded_absolute_path(path) {
        Ok(expanded) => expanded,
        Err(error) => {
            let mut output = OperationOutput::default();
            output.push_failure(Failure::file(
                OperationKind::Edit,
                display_path,
                error.to_string(),
            ));
            return output;
        }
    };
    batch::execute(
        service,
        vec![FileHunk::Update {
            path: expanded_path,
            move_path: None,
            chunks: vec![UpdateChunk {
                change_context: None,
                old_lines: replacement_lines(old_string),
                new_lines: replacement_lines(new_string),
                is_end_of_file: false,
            }],
        }],
    )
}
fn expanded_absolute_path(path: &str) -> anyhow::Result<std::path::PathBuf> {
    let expanded_path = crate::path_expansion::expand_path(path)?;
    anyhow::ensure!(
        expanded_path.is_absolute(),
        "path must be absolute after expansion"
    );
    Ok(expanded_path)
}
fn replacement_lines(value: &str) -> ChunkLines {
    value.lines().map(str::to_owned).collect()
}
fn success(mutation: &Mutation, uuid: OperationId) -> Success {
    let (before, after) = logical_stats(mutation);
    Success::new(
        mutation.kind,
        uuid,
        None,
        mutation.display_path.clone(),
        (mutation.kind != OperationKind::Add)
            .then_some(before)
            .flatten(),
        (mutation.kind != OperationKind::Delete)
            .then_some(after)
            .flatten(),
    )
}
pub(super) fn logical_stats(mutation: &Mutation) -> (Option<FileStats>, Option<FileStats>) {
    let before = mutation
        .change(PathRole::Source)
        .or_else(|_| mutation.change(PathRole::Single))
        .ok()
        .and_then(|change| change.before.contents())
        .map(FileStats::from_contents);
    let after = mutation
        .change(PathRole::Destination)
        .or_else(|_| mutation.change(PathRole::Single))
        .ok()
        .and_then(|change| change.after.contents())
        .map(FileStats::from_contents);
    (before, after)
}
