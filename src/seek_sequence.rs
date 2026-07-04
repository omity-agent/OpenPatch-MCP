use alloc::borrow::Cow;
mod normalize;
mod tier;
use normalize::normalize;
use tier::SearchTier;
pub(crate) struct LineSearchIndex<'slice, 'text> {
    lines: &'slice [&'text str],
    trim: Option<SearchTier<'text>>,
    normalized: Option<SearchTier<'text>>,
    ignore_empty_lines: Option<SearchTier<'text>>,
    collapse_spaces: Option<SearchTier<'text>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SequenceMatch {
    pub(crate) start: usize,
    pub(crate) length: usize,
}
impl<'slice, 'text> LineSearchIndex<'slice, 'text> {
    #[must_use]
    pub(crate) const fn new(lines: &'slice [&'text str]) -> Self {
        Self {
            lines,
            trim: None,
            normalized: None,
            ignore_empty_lines: None,
            collapse_spaces: None,
        }
    }
    #[must_use]
    pub(crate) fn seek(
        &mut self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        find_exact(self.lines, pattern, start, eof)
            .or_else(|| self.find(pattern, start, eof, MatchMode::Trim))
            .or_else(|| self.find(pattern, start, eof, MatchMode::Normalized))
            .or_else(|| self.find(pattern, start, eof, MatchMode::IgnoreEmptyLines))
            .or_else(|| self.find(pattern, start, eof, MatchMode::CollapseSpaces))
    }
    fn find(
        &mut self,
        pattern: &[String],
        start: usize,
        eof: bool,
        mode: MatchMode,
    ) -> Option<SequenceMatch> {
        let tier = self.tier(mode);
        tier.find(pattern, start, eof)
    }
    fn tier(&mut self, mode: MatchMode) -> &SearchTier<'text> {
        let lines = self.lines;
        match mode {
            MatchMode::Trim => self
                .trim
                .get_or_insert_with(|| SearchTier::new(lines, MatchMode::Trim)),
            MatchMode::Normalized => self
                .normalized
                .get_or_insert_with(|| SearchTier::new(lines, MatchMode::Normalized)),
            MatchMode::IgnoreEmptyLines => self
                .ignore_empty_lines
                .get_or_insert_with(|| SearchTier::new(lines, MatchMode::IgnoreEmptyLines)),
            MatchMode::CollapseSpaces => self
                .collapse_spaces
                .get_or_insert_with(|| SearchTier::new(lines, MatchMode::CollapseSpaces)),
        }
    }
}
fn find_exact(
    lines: &[&str],
    pattern: &[String],
    start: usize,
    eof: bool,
) -> Option<SequenceMatch> {
    if pattern.is_empty() {
        return Some(SequenceMatch { start, length: 0 });
    }
    if pattern.len() > lines.len() {
        return None;
    }
    if pattern.len() == 1
        && let Some(single_line) = pattern.first()
    {
        return find_exact_line(lines, single_line, start, eof);
    }
    let last_start = lines.len() - pattern.len();
    if eof {
        return window_matches_exact(lines, pattern, last_start).then_some(SequenceMatch {
            start: last_start,
            length: pattern.len(),
        });
    }
    if start > last_start {
        return None;
    }
    for candidate_start in start..=last_start {
        if window_matches_exact(lines, pattern, candidate_start) {
            return Some(SequenceMatch {
                start: candidate_start,
                length: pattern.len(),
            });
        }
    }
    None
}
fn find_exact_line(
    lines: &[&str],
    pattern: &str,
    start: usize,
    eof: bool,
) -> Option<SequenceMatch> {
    if eof {
        let last_start = lines.len() - 1;
        return (lines.get(last_start).copied() == Some(pattern)).then_some(SequenceMatch {
            start: last_start,
            length: 1,
        });
    }
    for index in start..lines.len() {
        if lines.get(index).copied() == Some(pattern) {
            return Some(SequenceMatch {
                start: index,
                length: 1,
            });
        }
    }
    None
}
fn window_matches_exact(lines: &[&str], pattern: &[String], start: usize) -> bool {
    let Some(end) = start.checked_add(pattern.len()) else {
        return false;
    };
    let Some(window) = lines.get(start..end) else {
        return false;
    };
    let Some((first_pattern, remaining_pattern)) = pattern.split_first() else {
        return true;
    };
    if window.first().copied() != Some(first_pattern.as_str()) {
        return false;
    }
    window
        .iter()
        .skip(1)
        .zip(remaining_pattern)
        .all(|(line, pattern_line)| *line == pattern_line)
}
#[derive(Clone, Copy)]
pub(super) enum MatchMode {
    Trim,
    Normalized,
    IgnoreEmptyLines,
    CollapseSpaces,
}
impl MatchMode {
    pub(super) fn key(self, source: &str) -> Cow<'_, str> {
        match self {
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
mod tests;
