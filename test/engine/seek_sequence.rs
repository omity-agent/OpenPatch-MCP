use super::LineSearchIndex;
#[test]
fn ordered_matches_preserve_physical_spans_and_search_bounds() {
    for (source, pattern, start, eof, expected) in [
        ("  target\nmiddle\ntarget", "target", 0, false, (2, 1)),
        ("target   \n  target", "target", 0, false, (0, 1)),
        ("a—b\n  a-b  ", "a-b", 0, false, (1, 1)),
        ("a—b\nc\na-b\n\nc", "a-b\nc", 0, false, (0, 2)),
        ("a\n\nb", "a\nb", 0, false, (0, 3)),
        ("a  b\n\nc   d", "a b\nc d", 0, false, (0, 3)),
        ("target\nmiddle\ntarget", "target", 0, true, (2, 1)),
        ("target\nmiddle\ntarget", "target", 1, false, (2, 1)),
        (
            "same\nsame\nsame\nunique\nsame\nsame",
            "same\nunique\nsame",
            0,
            false,
            (2, 3),
        ),
    ] {
        let lines = source.split('\n').collect::<Vec<_>>();
        let pattern_lines = pattern.split('\n').map(str::to_owned).collect::<Vec<_>>();
        let actual = LineSearchIndex::new(&lines).seek(&pattern_lines, start, eof);
        assert_eq!(
            actual.map(|found| (found.start, found.length)),
            Some(expected),
            "{source:?}: {pattern:?}"
        );
    }
}
#[test]
fn exact_search_does_not_accept_a_trimmed_match() {
    let lines = [" target "];
    let pattern = [String::from("target")];
    assert_eq!(
        LineSearchIndex::new(&lines).seek_exact(&pattern, 0, false),
        None
    );
}
#[test]
fn closest_matches_use_unicode_alignment_scores_and_physical_spans() {
    for (source, pattern, expected) in [
        ("abzzef\nabcxef", "abcdef", Some((1, 1))),
        ("one\ntwo\nthree", "twx", Some((1, 1))),
        ("alpha\nx\nbeta", "alpha\nbeta", Some((0, 3))),
        ("unrelated\n你好世畀", "你好世界", Some((1, 1))),
        ("abxy", "abcd", None),
    ] {
        let lines = source.split('\n').collect::<Vec<_>>();
        let pattern_lines = pattern.split('\n').map(str::to_owned).collect::<Vec<_>>();
        let actual = LineSearchIndex::new(&lines).closest(&pattern_lines);
        assert_eq!(actual.map(|found| (found.start, found.length)), expected);
    }
}
