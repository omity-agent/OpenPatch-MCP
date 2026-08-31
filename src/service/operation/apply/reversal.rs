use super::{
    super::model::{FileState, Mutation, OperationKind, PathChange, PathRole, StoredOperation},
    files, merge,
};
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
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the three file contents must remain borrowed during merge planning"
)]
fn plan_move(stored: &StoredOperation) -> anyhow::Result<Mutation> {
    let source = stored.change(PathRole::Source)?;
    let destination = stored.change(PathRole::Destination)?;
    let current_source = files::snapshot(&source.path, "Failed to inspect move source for undo")?;
    if current_source != source.after {
        anyhow::bail!("move source path changed after the recorded operation");
    }
    let current_destination =
        files::snapshot(&destination.path, "Failed to read moved file for undo")?;
    let FileState::Present(current_contents) = &current_destination else {
        anyhow::bail!("moved file no longer exists at its destination");
    };
    let FileState::Present(after_contents) = &destination.after else {
        anyhow::bail!("move history has no destination artifact");
    };
    let FileState::Present(before_contents) = &source.before else {
        anyhow::bail!("move history has no source artifact");
    };
    let restored = merge::reverse_contents(before_contents, after_contents, current_contents)?;
    Ok(Mutation {
        kind: OperationKind::Edit,
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
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed states avoids cloning before the selected transition is known"
)]
fn reverse_state(
    before: &FileState,
    after: &FileState,
    current: &FileState,
) -> anyhow::Result<FileState> {
    match (before, after, current) {
        (
            FileState::Present(before_contents),
            FileState::Present(after_contents),
            FileState::Present(current_contents),
        ) => Ok(FileState::present(merge::reverse_contents(
            before_contents,
            after_contents,
            current_contents,
        )?)),
        (
            FileState::Missing,
            FileState::Present(after_contents),
            FileState::Present(current_contents),
        ) if after_contents == current_contents => Ok(FileState::Missing),
        (FileState::Missing, FileState::Present(_), FileState::Present(_)) => {
            anyhow::bail!("added file was modified after the recorded operation")
        }
        (FileState::Missing, FileState::Present(_), FileState::Missing) => {
            anyhow::bail!("added file is already absent")
        }
        (FileState::Present(before_contents), FileState::Missing, FileState::Missing) => {
            Ok(FileState::share(before_contents))
        }
        (FileState::Present(_), FileState::Missing, FileState::Present(_)) => {
            anyhow::bail!("deleted path was recreated after the recorded operation")
        }
        _ => anyhow::bail!("unsupported or corrupt operation state transition"),
    }
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "the operation kind is derived directly from two borrowed states"
)]
const fn transition_kind(before: &FileState, after: &FileState) -> OperationKind {
    match (before, after) {
        (FileState::Missing, FileState::Present(_)) => OperationKind::Add,
        (FileState::Present(_), FileState::Missing) => OperationKind::Delete,
        _ => OperationKind::Edit,
    }
}
