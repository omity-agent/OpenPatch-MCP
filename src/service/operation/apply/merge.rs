use crate::text::LineEndings;
use anyhow::Context as _;
use diffy::DiffOptions;
const CONFLICT_REASON: &str = "current file changes conflict with the operation being undone";
pub(super) fn reverse_contents(before: &str, after: &str, current: &str) -> anyhow::Result<String> {
    let line_endings = LineEndings::detect(current);
    let normalized_before = LineEndings::normalize(before);
    let normalized_after = LineEndings::normalize(after);
    let normalized_current = LineEndings::normalize(current);
    let reversed = reverse_normalized(&normalized_before, &normalized_after, &normalized_current)?;
    Ok(line_endings.render(reversed))
}
fn reverse_normalized(before: &str, after: &str, current: &str) -> anyhow::Result<String> {
    if after == before {
        return Ok(current.to_owned());
    }
    if current == before {
        anyhow::bail!("operation is already undone");
    }
    let mut options = DiffOptions::new();
    options.set_context_len(0);
    let reversed = transfer_changes(&options, after, before, current)?;
    let locally_modified_before = transfer_changes(&options, after, current, before)?;
    if reversed != locally_modified_before {
        anyhow::bail!(CONFLICT_REASON);
    }
    Ok(reversed)
}
fn transfer_changes(
    options: &DiffOptions,
    source: &str,
    target: &str,
    destination: &str,
) -> anyhow::Result<String> {
    let patch = options.create_patch(source, target);
    diffy::apply(destination, &patch).context(CONFLICT_REASON)
}
#[cfg(test)]
#[path = "../../../../test/service/merge.rs"]
mod tests;
