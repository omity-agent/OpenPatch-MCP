use alloc::borrow::Cow;
use core::cell::OnceCell;
use enum_map::{Enum, EnumMap};
mod nearest;
mod normalize;
mod tier;
use normalize::normalize;
use tier::SearchTier;
pub(crate) struct LineSearchIndex<'slice, 'text> {
    lines: &'slice [&'text str],
    tiers: EnumMap<MatchMode, OnceCell<SearchTier<'text>>>,
    nearest: OnceCell<nearest::SearchIndex>,
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
            tiers: EnumMap::default(),
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
        self.tiers
            .iter()
            .find_map(|(mode, _)| self.find(pattern, start, eof, mode))
    }
    #[must_use]
    pub(crate) fn seek_exact(
        &self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        self.find(pattern, start, eof, MatchMode::Exact)
    }
    #[must_use]
    pub(crate) fn closest(&self, pattern: &[String]) -> Option<SequenceMatch> {
        self.nearest
            .get_or_init(|| nearest::SearchIndex::new(self.lines))
            .find(pattern)
    }
    fn find(
        &self,
        pattern: &[String],
        start: usize,
        eof: bool,
        mode: MatchMode,
    ) -> Option<SequenceMatch> {
        if pattern.is_empty() {
            return Some(SequenceMatch { start, length: 0 });
        }
        let tier = self.tiers[mode].get_or_init(|| SearchTier::new(self.lines, mode));
        tier.find(pattern, start, eof)
    }
}
#[derive(Clone, Copy, Enum)]
pub(super) enum MatchMode {
    Exact,
    Trim,
    Normalized,
    IgnoreEmptyLines,
    CollapseSpaces,
}
impl MatchMode {
    pub(super) fn key(self, source: &str) -> Cow<'_, str> {
        match self {
            Self::Exact => Cow::Borrowed(source),
            Self::Trim => Cow::Borrowed(source.trim()),
            Self::Normalized | Self::IgnoreEmptyLines => normalize(source),
            Self::CollapseSpaces => normalize::collapse_spaces(source),
        }
    }
    pub(super) const fn skips_empty_lines(self) -> bool {
        matches!(self, Self::IgnoreEmptyLines | Self::CollapseSpaces)
    }
}
#[cfg(test)]
#[path = "../../test/engine/seek_sequence.rs"]
mod tests;
