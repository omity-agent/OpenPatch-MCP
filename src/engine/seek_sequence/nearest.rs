mod alignment;
mod corpus;
mod scoring;
use self::{alignment::fit, corpus::NormalizedCorpus, scoring::MIN_ACCEPTED_SCORE};
use super::{SequenceMatch, normalize::collapse_spaces};
pub(super) struct SearchIndex {
    target: NormalizedCorpus,
}
impl SearchIndex {
    pub(super) fn new(lines: &[&str]) -> Self {
        Self {
            target: NormalizedCorpus::new(lines),
        }
    }
    pub(super) fn find(&self, pattern: &[String]) -> Option<SequenceMatch> {
        let pattern_chars = flatten_pattern(pattern);
        if pattern_chars.is_empty() || self.target.is_empty() {
            return None;
        }
        let alignment = fit(&pattern_chars, self.target.chars());
        (alignment.score > MIN_ACCEPTED_SCORE)
            .then(|| self.target.sequence(alignment.start, alignment.end))
            .flatten()
    }
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
#[path = "../../../test/engine/nearest.rs"]
mod tests;
