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
