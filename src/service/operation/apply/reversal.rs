use super::{
    super::model::{FileState, Mutation, OperationKind, PathChange, PathRole, StoredOperation},
    files, merge,
};
use anyhow::Context as _;
use smallvec::smallvec;
pub(super) fn plan(stored: &StoredOperation) -> anyhow::Result<Mutation> {
    if stored.changes.len() == 1 {
        plan_single(stored)
    } else {
        plan_move(stored)
    }
}
fn plan_single(stored: &StoredOperation) -> anyhow::Result<Mutation> {
    let change = stored
        .changes
        .first()
        .ok_or_else(|| anyhow::anyhow!("operation history has no file state"))?;
    let current = files::snapshot(&change.path, "Failed to read current file for undo")?;
    let result = reverse_state(&change.before, &change.after, &current)?;
    let kind = transition_kind(&current, &result);
    Ok(Mutation::single(kind, change.path.clone(), current, result))
}
fn plan_move(stored: &StoredOperation) -> anyhow::Result<Mutation> {
    let source = stored.change(PathRole::Source)?;
    let destination = stored.change(PathRole::Destination)?;
    let current_source = files::snapshot(&source.path, "Failed to inspect move source for undo")?;
    if current_source != source.after {
        anyhow::bail!("move source path changed after the recorded operation");
    }
    let current_destination =
        files::snapshot(&destination.path, "Failed to read moved file for undo")?;
    let current_contents = current_destination
        .contents()
        .context("moved file no longer exists at its destination")?;
    let after_contents = destination
        .after
        .contents()
        .context("move history has no destination artifact")?;
    let before_contents = source
        .before
        .contents()
        .context("move history has no source artifact")?;
    let restored = merge::reverse_contents(before_contents, after_contents, current_contents)?;
    Ok(Mutation {
        kind: OperationKind::Move,
        display_path: source.path.clone(),
        changes: smallvec![
            PathChange {
                role: PathRole::Destination,
                path: source.path.clone(),
                before: current_source,
                after: FileState::present(restored),
            },
            PathChange {
                role: PathRole::Source,
                path: destination.path.clone(),
                before: current_destination,
                after: destination.before.clone(),
            },
        ],
    })
}
fn reverse_state(
    before: &FileState,
    after: &FileState,
    current: &FileState,
) -> anyhow::Result<FileState> {
    match (before.contents(), after.contents(), current.contents()) {
        (Some(before_contents), Some(after_contents), Some(current_contents)) => {
            Ok(FileState::present(merge::reverse_contents(
                before_contents,
                after_contents,
                current_contents,
            )?))
        }
        (Some(_), Some(_), None) => {
            anyhow::bail!("edited file no longer exists at its recorded path")
        }
        (None, Some(after_contents), Some(current_contents))
            if after_contents == current_contents =>
        {
            Ok(FileState::Missing)
        }
        (None, Some(_), Some(_)) => {
            anyhow::bail!("added file was modified after the recorded operation")
        }
        (None, Some(_), None) => {
            anyhow::bail!("added file is already absent")
        }
        (Some(_), None, None) => Ok(before.clone()),
        (Some(_), None, Some(_)) => {
            anyhow::bail!("deleted path was recreated after the recorded operation")
        }
        _ => anyhow::bail!("unsupported or corrupt operation state transition"),
    }
}
const fn transition_kind(before: &FileState, after: &FileState) -> OperationKind {
    match (before, after) {
        (&FileState::Missing, &FileState::Present(_)) => OperationKind::Add,
        (&FileState::Present(_), &FileState::Missing) => OperationKind::Delete,
        _ => OperationKind::Edit,
    }
}
