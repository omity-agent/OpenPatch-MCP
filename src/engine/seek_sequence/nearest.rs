mod alignment;
mod corpus;
mod scoring;
use self::{alignment::fit, corpus::NormalizedCorpus, scoring::MIN_ACCEPTED_SCORE};
use super::{SequenceMatch, normalize::collapse_spaces};
pub(super) fn find(lines: &[&str], pattern: &[String]) -> Option<SequenceMatch> {
    let pattern_chars = flatten_pattern(pattern);
    if pattern_chars.is_empty() || lines.is_empty() {
        return None;
    }
    let target = NormalizedCorpus::new(lines);
    if target.is_empty() {
        return None;
    }
    let alignment = fit(&pattern_chars, target.chars());
    (alignment.score > MIN_ACCEPTED_SCORE)
        .then(|| target.sequence(alignment.start, alignment.end))
        .flatten()
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
