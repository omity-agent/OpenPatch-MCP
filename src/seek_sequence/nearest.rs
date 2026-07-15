mod corpus;
mod ranking;
use self::{
    corpus::FlattenedLines,
    ranking::{Candidate, Similarity, THRESHOLD},
};
use super::{SequenceMatch, normalize::collapse_spaces};
use rapidfuzz::distance::levenshtein;
pub(super) fn find(lines: &[&str], pattern: &[String]) -> Option<SequenceMatch> {
    let pattern_chars = flatten_pattern(pattern);
    if pattern_chars.is_empty() || lines.is_empty() {
        return None;
    }
    let target = FlattenedLines::new(lines);
    let scorer = levenshtein::BatchComparator::new(pattern_chars.iter().copied());
    let mut best: Option<Candidate> = None;
    for start in 0..target.entry_count() {
        let candidate_range = target.candidate_range(start, pattern_chars.len());
        let pivot = target.closest_length_index(start, pattern_chars.len(), &candidate_range);
        if let Some(pivot_index) = pivot
            && let Some(exact) = evaluate(
                &target,
                &scorer,
                start,
                pivot_index,
                pattern_chars.len(),
                &mut best,
            )
        {
            return Some(exact);
        }
        for end in candidate_range {
            if Some(end) == pivot {
                continue;
            }
            if let Some(exact) =
                evaluate(&target, &scorer, start, end, pattern_chars.len(), &mut best)
            {
                return Some(exact);
            }
        }
    }
    best.map(|candidate| candidate.sequence)
}
fn evaluate(
    target: &FlattenedLines,
    scorer: &levenshtein::BatchComparator<char>,
    start: usize,
    end: usize,
    pattern_length: usize,
    best: &mut Option<Candidate>,
) -> Option<SequenceMatch> {
    let fragment = target.fragment(start, end);
    let length_bound = Similarity::length_bound(pattern_length, fragment.len());
    if best
        .as_ref()
        .is_some_and(|current| length_bound.compare(current.similarity).is_lt())
    {
        return None;
    }
    let maximum = pattern_length.max(fragment.len());
    let cutoff = best.as_ref().map_or_else(
        || THRESHOLD.strict_distance_cutoff(maximum),
        |current| current.similarity.distance_cutoff(maximum),
    );
    let arguments = levenshtein::Args::default().score_cutoff(cutoff);
    let distance = scorer.distance_with_args(fragment.iter().copied(), &arguments)?;
    let candidate = Candidate::new(
        target.sequence(start, end),
        pattern_length,
        fragment.len(),
        distance,
    );
    if candidate.similarity.is_exact() {
        return Some(candidate.sequence);
    }
    if best
        .as_ref()
        .is_none_or(|current| candidate.is_better_than(current))
    {
        *best = Some(candidate);
    }
    None
}
fn flatten_pattern(pattern: &[String]) -> Vec<char> {
    let mut chars = Vec::new();
    for line in pattern {
        let normalized = collapse_spaces(line);
        if normalized.is_empty() {
            continue;
        }
        if !chars.is_empty() {
            chars.push('\n');
        }
        chars.extend(normalized.chars());
    }
    chars
}
#[cfg(test)]
mod tests;
