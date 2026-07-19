use super::{LineSearchIndex, SequenceMatch};
fn pattern(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
fn seek_sequence(lines: &[&str], pattern: &[String], start: usize, eof: bool) -> Option<usize> {
    LineSearchIndex::new(lines)
        .seek(pattern, start, eof)
        .map(|sequence_match| sequence_match.start)
}
fn seek_span(lines: &[&str], pattern: &[String], start: usize, eof: bool) -> Option<SequenceMatch> {
    LineSearchIndex::new(lines).seek(pattern, start, eof)
}
#[test]
fn exact_match_keeps_priority_over_earlier_trim_match() {
    let source = ["  target", "middle", "target"];
    let pattern = pattern(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(2));
}
#[test]
fn trim_match_replaces_removed_trim_end_tier() {
    let source = ["target   ", "  target"];
    let pattern = pattern(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(0));
}
#[test]
fn trim_match_keeps_priority_over_earlier_normalized_match() {
    let source = ["a—b", "  a-b  "];
    let pattern = pattern(&["a-b"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(1));
}
#[test]
fn normalized_match_keeps_priority_over_earlier_empty_line_match() {
    let source = ["a—b", "c", "a-b", "", "c"];
    let pattern = pattern(&["a-b", "c"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, false), Some(0));
}
#[test]
fn empty_lines_can_be_ignored() {
    let source = ["a", "", "b"];
    let pattern = pattern(&["a", "b"]);
    assert_eq!(
        seek_span(&source, &pattern, 0, false),
        Some(SequenceMatch {
            start: 0,
            length: 3
        })
    );
}
#[test]
fn space_runs_can_be_collapsed_after_empty_lines() {
    let source = ["a  b", "", "c   d"];
    let pattern = pattern(&["a b", "c d"]);
    assert_eq!(
        seek_span(&source, &pattern, 0, false),
        Some(SequenceMatch {
            start: 0,
            length: 3
        })
    );
}
#[test]
fn eof_search_only_checks_the_last_possible_window() {
    let source = ["target", "middle", "target"];
    let pattern = pattern(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 0, true), Some(2));
}
#[test]
fn start_index_excludes_earlier_candidates() {
    let source = ["target", "middle", "target"];
    let pattern = pattern(&["target"]);
    assert_eq!(seek_sequence(&source, &pattern, 1, false), Some(2));
}
#[test]
fn exact_search_does_not_accept_a_trimmed_match() {
    let lines = [" target "];
    let pattern = pattern(&["target"]);
    assert_eq!(
        LineSearchIndex::new(&lines).seek_exact(&pattern, 0, false),
        None
    );
}
#[test]
fn rare_middle_line_can_anchor_the_match() {
    let source = ["same", "same", "same", "unique", "same", "same"];
    let pattern = pattern(&["same", "unique", "same"]);
    let mut index = LineSearchIndex::new(&source);
    assert_eq!(
        index.seek(&pattern, 0, false),
        Some(SequenceMatch {
            start: 2,
            length: 3
        })
    );
}
#[test]
fn closest_match_uses_the_highest_affine_alignment_score() {
    let source = ["abzzef", "abcxef"];
    let pattern = pattern(&["abcdef"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(
        index.closest(&pattern),
        Some(SequenceMatch {
            start: 1,
            length: 1
        })
    );
}
#[test]
fn closest_match_finds_a_single_substitution() {
    let source = ["one", "two", "three"];
    let pattern = pattern(&["twx"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(
        index.closest(&pattern),
        Some(SequenceMatch {
            start: 1,
            length: 1
        })
    );
}
#[test]
fn closest_match_can_span_a_different_number_of_lines() {
    let source = ["alpha", "x", "beta"];
    let pattern = pattern(&["alpha", "beta"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(
        index.closest(&pattern),
        Some(SequenceMatch {
            start: 0,
            length: 3
        })
    );
}
#[test]
fn closest_match_compares_unicode_characters() {
    let source = ["unrelated", "你好世畀"];
    let pattern = pattern(&["你好世界"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(
        index.closest(&pattern),
        Some(SequenceMatch {
            start: 1,
            length: 1
        })
    );
}
#[test]
fn closest_match_must_be_strictly_above_threshold() {
    let source = ["abxy"];
    let pattern = pattern(&["abcd"]);
    let index = LineSearchIndex::new(&source);
    assert_eq!(index.closest(&pattern), None);
}
