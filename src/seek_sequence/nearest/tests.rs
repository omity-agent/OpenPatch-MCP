use super::{alignment::Alignment, alignment::fit, find};
use crate::seek_sequence::SequenceMatch;
fn chars(value: &str) -> Vec<char> {
    value.chars().collect()
}
#[test]
fn fitting_alignment_consumes_the_pattern_and_skips_target_ends() {
    let pattern = chars("abcYdef");
    let target = chars("prefix abcXdef suffix");
    assert_eq!(
        fit(&pattern, &target),
        Alignment {
            score: 15,
            start: 7,
            end: 14,
        }
    );
}
#[test]
fn a_contiguous_pattern_gap_uses_one_opening_penalty() {
    let pattern = chars("abcXYZdef");
    let target = chars("abcdef");
    assert_eq!(
        fit(&pattern, &target),
        Alignment {
            score: 11,
            start: 0,
            end: 6,
        }
    );
}
#[test]
fn a_contiguous_target_gap_uses_one_opening_penalty() {
    let pattern = chars("abcdef");
    let target = chars("abcXYZdef");
    assert_eq!(
        fit(&pattern, &target),
        Alignment {
            score: 11,
            start: 0,
            end: 9,
        }
    );
}
#[test]
fn character_boundaries_map_back_to_the_enclosing_original_lines() {
    let lines = [
        "unrelated prefix",
        "",
        "before abcXdef after",
        "unrelated suffix",
    ];
    let pattern = [String::from("abcYdef")];
    assert_eq!(
        find(&lines, &pattern),
        Some(SequenceMatch {
            start: 2,
            length: 1,
        })
    );
}
