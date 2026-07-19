use core::cmp::Reverse;
use diffy::Line;
#[derive(Debug, Clone)]
struct Change {
    start: usize,
    old: Vec<String>,
    new: Vec<String>,
}
pub(super) fn reverse_contents(before: &str, after: &str, current: &str) -> anyhow::Result<String> {
    if after == before {
        return Ok(current.to_owned());
    }
    if current == before {
        anyhow::bail!("operation is already undone");
    }
    let undo_changes = changes(after, before);
    let local_changes = changes(after, current);
    for undo_change in &undo_changes {
        if local_changes
            .iter()
            .any(|local_change| overlaps(undo_change, local_change))
        {
            anyhow::bail!("current file changes conflict with the operation being undone");
        }
    }
    let mut planned = Vec::with_capacity(undo_changes.len());
    for undo_change in undo_changes {
        planned.push((mapped_start(&undo_change, &local_changes)?, undo_change));
    }
    planned.sort_unstable_by_key(|item| Reverse(item.0));
    let mut current_lines = lines(current);
    for (start, change) in planned {
        let end = start
            .checked_add(change.old.len())
            .ok_or_else(|| anyhow::anyhow!("undo line range overflowed"))?;
        let actual = current_lines
            .get(start..end)
            .ok_or_else(|| anyhow::anyhow!("expected undo lines are no longer present"))?;
        if actual != change.old {
            anyhow::bail!("expected undo lines were modified after the recorded operation");
        }
        current_lines.splice(start..end, change.new);
    }
    Ok(current_lines.concat())
}
#[expect(
    clippy::pattern_type_mismatch,
    reason = "diffy exposes borrowed line variants"
)]
fn changes(base: &str, target: &str) -> Vec<Change> {
    let patch = diffy::create_patch(base, target);
    let mut changes = Vec::new();
    for hunk in patch.hunks() {
        let mut base_index = hunk.old_range().start().saturating_sub(1);
        let mut active: Option<Change> = None;
        for line in hunk.lines() {
            match line {
                Line::Context(_) => {
                    finish(&mut changes, &mut active);
                    base_index = base_index.saturating_add(1);
                }
                Line::Delete(text) => {
                    let change = active.get_or_insert_with(|| Change {
                        start: base_index,
                        old: Vec::new(),
                        new: Vec::new(),
                    });
                    change.old.push((*text).to_owned());
                    base_index = base_index.saturating_add(1);
                }
                Line::Insert(text) => {
                    let change = active.get_or_insert_with(|| Change {
                        start: base_index,
                        old: Vec::new(),
                        new: Vec::new(),
                    });
                    change.new.push((*text).to_owned());
                }
            }
        }
        finish(&mut changes, &mut active);
    }
    changes
}
fn finish(changes: &mut Vec<Change>, active: &mut Option<Change>) {
    if let Some(change) = active.take() {
        changes.push(change);
    }
}
const fn overlaps(left: &Change, right: &Change) -> bool {
    let left_end = left.start.saturating_add(left.old.len());
    let right_end = right.start.saturating_add(right.old.len());
    match (left.old.is_empty(), right.old.is_empty()) {
        (true, true) => left.start == right.start,
        (true, false) => right.start < left.start && left.start < right_end,
        (false, true) => left.start < right.start && right.start < left_end,
        (false, false) => left.start < right_end && right.start < left_end,
    }
}
fn mapped_start(change: &Change, local_changes: &[Change]) -> anyhow::Result<usize> {
    let mut mapped = i64::try_from(change.start)?;
    for local in local_changes {
        let local_end = local.start.saturating_add(local.old.len());
        let precedes = if local.old.is_empty() {
            local.start <= change.start
        } else {
            local_end <= change.start
        };
        if precedes {
            let new_length = i64::try_from(local.new.len())?;
            let old_length = i64::try_from(local.old.len())?;
            mapped = mapped
                .checked_add(new_length - old_length)
                .ok_or_else(|| anyhow::anyhow!("undo line mapping overflowed"))?;
        }
    }
    usize::try_from(mapped).map_err(Into::into)
}
fn lines(contents: &str) -> Vec<String> {
    contents.split_inclusive('\n').map(str::to_owned).collect()
}
#[cfg(test)]
mod tests {
    use super::reverse_contents;
    #[test]
    fn preserves_an_adjacent_unrelated_edit() {
        let result = reverse_contents("old\nkeep\n", "new\nkeep\n", "new\nchanged\n").unwrap();
        assert_eq!(result, "old\nchanged\n");
    }
    #[test]
    fn rejects_an_overlapping_edit() {
        let error = reverse_contents("old\n", "new\n", "modified\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "current file changes conflict with the operation being undone"
        );
    }
}
