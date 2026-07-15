use super::{Candidate, FlattenedLines, THRESHOLD, find, flatten_pattern};
use rapidfuzz::distance::levenshtein;
#[test]
fn rapidfuzz_cutoff_includes_boundary_distance() {
    let scorer = levenshtein::BatchComparator::new("twx".chars());
    let exact = levenshtein::Args::default().score_cutoff(0);
    let one_edit = levenshtein::Args::default().score_cutoff(1);
    assert_eq!(Some(0), scorer.distance_with_args("twx".chars(), &exact));
    assert_eq!(Some(1), scorer.distance_with_args("two".chars(), &one_edit));
}
#[test]
fn candidate_window_keeps_equal_length_fragments() {
    let target = FlattenedLines::new(&["one", "two", "three"]);
    let range = target.candidate_range(1, 3);
    assert_eq!(range, 1..2);
    assert_eq!(target.closest_length_index(1, 3, &range), Some(1));
    assert_eq!(target.fragment(1, 1), ['t', 'w', 'o']);
}
#[test]
fn optimized_search_matches_exhaustive_reference() {
    let values = ["", "a", "b", "ab"];
    for first in values {
        for second in values {
            for third in values {
                let lines = [first, second, third];
                for pattern_first in values {
                    for pattern_second in values {
                        let pattern = [pattern_first.to_owned(), pattern_second.to_owned()];
                        assert_eq!(
                            find(&lines, &pattern),
                            exhaustive(&lines, &pattern),
                            "lines={lines:?}, pattern={pattern:?}"
                        );
                    }
                }
            }
        }
    }
}
fn exhaustive(lines: &[&str], pattern: &[String]) -> Option<super::SequenceMatch> {
    let pattern_chars = flatten_pattern(pattern);
    if pattern_chars.is_empty() {
        return None;
    }
    let target = FlattenedLines::new(lines);
    let scorer = levenshtein::BatchComparator::new(pattern_chars.iter().copied());
    let mut best: Option<Candidate> = None;
    for start in 0..target.entry_count() {
        for end in start..target.entry_count() {
            let fragment = target.fragment(start, end);
            let distance = scorer.distance(fragment.iter().copied());
            let candidate = Candidate::new(
                target.sequence(start, end),
                pattern_chars.len(),
                fragment.len(),
                distance,
            );
            if candidate.similarity.compare(THRESHOLD).is_gt()
                && best
                    .as_ref()
                    .is_none_or(|current| candidate.is_better_than(current))
            {
                best = Some(candidate);
            }
        }
    }
    best.map(|candidate| candidate.sequence)
}
