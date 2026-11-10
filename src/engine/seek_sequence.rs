use core::cell::OnceCell;
mod index;
mod nearest;
mod rules;
use index::SequenceIndex;
use rules::RuleChain;
pub(crate) struct LineSearchIndex<'slice, 'text> {
    lines: &'slice [&'text str],
    indexes: Vec<RuleIndex<'text>>,
    nearest: OnceCell<nearest::SearchIndex>,
}
struct RuleIndex<'text> {
    chain: RuleChain,
    cache: OnceCell<SequenceIndex<'text>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SequenceMatch {
    pub(crate) start: usize,
    pub(crate) length: usize,
}
impl<'slice, 'text> LineSearchIndex<'slice, 'text> {
    #[must_use]
    pub(crate) fn new(lines: &'slice [&'text str]) -> Self {
        Self {
            lines,
            indexes: RuleChain::prefixes()
                .map(|chain| RuleIndex {
                    chain,
                    cache: OnceCell::new(),
                })
                .collect(),
            nearest: OnceCell::new(),
        }
    }
    #[must_use]
    pub(crate) fn seek(
        &self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        self.indexes
            .iter()
            .find_map(|index| index.find(self.lines, pattern, start, eof))
    }
    #[must_use]
    pub(crate) fn seek_exact(
        &self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        self.indexes.first()?.find(self.lines, pattern, start, eof)
    }
    #[must_use]
    pub(crate) fn closest(&self, pattern: &[String]) -> Option<SequenceMatch> {
        self.nearest
            .get_or_init(|| nearest::SearchIndex::new(self.lines))
            .find(pattern)
    }
}
impl<'text> RuleIndex<'text> {
    fn find(
        &self,
        lines: &[&'text str],
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        if pattern.is_empty() {
            return Some(SequenceMatch { start, length: 0 });
        }
        self.cache
            .get_or_init(|| SequenceIndex::new(lines, self.chain))
            .find(pattern, start, eof)
    }
}
#[cfg(test)]
#[path = "../../test/engine/seek_sequence.rs"]
mod tests;
