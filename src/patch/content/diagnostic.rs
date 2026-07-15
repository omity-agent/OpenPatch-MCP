use crate::seek_sequence::LineSearchIndex;
pub(super) fn match_failure_reason(
    message: &str,
    original_lines: &[&str],
    search_index: &LineSearchIndex<'_, '_>,
    pattern: &[String],
) -> String {
    let Some(closest) = search_index.closest(pattern) else {
        return message.to_owned();
    };
    let Some(end) = closest.start.checked_add(closest.length) else {
        panic!("closest match range must be valid");
    };
    let Some(lines) = original_lines.get(closest.start..end) else {
        panic!("closest match range must reference original lines");
    };
    let target_fragment = lines
        .iter()
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>()
        .join("\n");
    format_closest_match(message, &target_fragment)
}
fn format_closest_match(message: &str, target_fragment: &str) -> String {
    let longest_run = target_fragment
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence_length = longest_run
        .checked_add(1)
        .unwrap_or_else(|| panic!("Markdown fence length must fit usize"))
        .max(3);
    let fence = "`".repeat(fence_length);
    format!("{message}. Closest match:\n{fence}\n{target_fragment}\n{fence}")
}
#[cfg(test)]
mod tests {
    use super::format_closest_match;
    #[test]
    fn closest_match_uses_a_standard_code_fence() {
        assert_eq!(
            format_closest_match("Failure", "actual"),
            "Failure. Closest match:\n```\nactual\n```"
        );
    }
    #[test]
    fn code_fence_is_longer_than_embedded_backtick_runs() {
        assert_eq!(
            format_closest_match("Failure", "before ```` after"),
            "Failure. Closest match:\n`````\nbefore ```` after\n`````"
        );
    }
}
