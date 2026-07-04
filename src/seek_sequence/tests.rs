use super::LineSearchIndex;
fn lines(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
fn seek_sequence(lines: &[String], pattern: &[String], start: usize, eof: bool) -> Option<usize> {
    LineSearchIndex::new(lines).seek(pattern, start, eof)
}
#[test]
fn exact_match_keeps_priority_over_earlier_trim_match() {
    let source = lines(&["  target", "middle", "target"]);
    let pattern = lines(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(2));
}
#[test]
fn trim_end_match_keeps_priority_over_earlier_trim_match() {
    let source = lines(&["  target", "target   "]);
    let pattern = lines(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(1));
}
#[test]
fn trim_match_keeps_priority_over_earlier_normalized_match() {
    let source = lines(&["a—b", "  a-b  "]);
    let pattern = lines(&["a-b"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(1));
}
#[test]
fn eof_search_only_checks_the_last_possible_window() {
    let source = lines(&["target", "middle", "target"]);
    let pattern = lines(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, true), Some(2));
}
#[test]
fn start_index_excludes_earlier_candidates() {
    let source = lines(&["target", "middle", "target"]);
    let pattern = lines(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 1, false), Some(2));
}
#[test]
fn rare_middle_line_can_anchor_the_match() {
    let source = lines(&["same", "same", "same", "unique", "same", "same"]);
    let pattern = lines(&["same", "unique", "same"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(index.seek(&pattern, 0, false), Some(2));
}
