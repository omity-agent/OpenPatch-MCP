use alloc::borrow::Cow;
use std::collections::{HashMap, hash_map::Entry};
mod normalize;
mod relaxed;
use normalize::normalize;
use relaxed::find_relaxed;
pub(crate) struct LineSearchIndex<'slice, 'text> {
    lines: &'slice [&'text str],
    exact: Option<SearchTier<'text>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SequenceMatch {
    pub(crate) start: usize,
    pub(crate) length: usize,
}
impl<'slice, 'text> LineSearchIndex<'slice, 'text> {
    #[must_use]
    pub(crate) const fn new(lines: &'slice [&'text str]) -> Self {
        Self { lines, exact: None }
    }
    #[must_use]
    pub(crate) fn seek(
        &mut self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        if let found @ Some(_) = self.find(pattern, start, eof, MatchMode::Exact) {
            return found;
        }
        if let found @ Some(_) = find_relaxed(self.lines, pattern, start, eof, MatchMode::Trim) {
            return found;
        }
        if let found @ Some(_) =
            find_relaxed(self.lines, pattern, start, eof, MatchMode::Normalized)
        {
            return found;
        }
        if let found @ Some(_) =
            find_relaxed(self.lines, pattern, start, eof, MatchMode::IgnoreEmptyLines)
        {
            return found;
        }
        find_relaxed(self.lines, pattern, start, eof, MatchMode::CollapseSpaces)
    }
    fn find(
        &mut self,
        pattern: &[String],
        start: usize,
        eof: bool,
        mode: MatchMode,
    ) -> Option<SequenceMatch> {
        let lines = self.lines;
        let tier = match mode {
            MatchMode::Exact => self
                .exact
                .get_or_insert_with(|| SearchTier::new(lines, MatchMode::Exact)),
            MatchMode::Trim
            | MatchMode::Normalized
            | MatchMode::IgnoreEmptyLines
            | MatchMode::CollapseSpaces => {
                panic!("only exact mode uses a persistent full index")
            }
        };
        tier.find(pattern, start, eof)
    }
}
#[derive(Clone, Copy)]
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
struct SearchTier<'text> {
    mode: MatchMode,
    line_keys: Vec<usize>,
    key_ids: HashMap<Cow<'text, str>, usize>,
    postings: Vec<Vec<usize>>,
}
impl<'text> SearchTier<'text> {
    fn new(lines: &[&'text str], mode: MatchMode) -> Self {
        let mut tier = Self {
            mode,
            line_keys: Vec::with_capacity(lines.len()),
            key_ids: HashMap::with_capacity(lines.len()),
            postings: Vec::new(),
        };
        for (index, line) in lines.iter().enumerate() {
            tier.push_line(index, line);
        }
        tier
    }
    fn push_line(&mut self, index: usize, line: &'text str) {
        let key = self.mode.key(line);
        let key_id = match self.key_ids.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let next_id = self.postings.len();
                entry.insert(next_id);
                self.postings.push(Vec::new());
                next_id
            }
        };
        self.line_keys.push(key_id);
        let Some(posting) = self.postings.get_mut(key_id) else {
            panic!("line key id must reference a posting list");
        };
        posting.push(index);
    }
    fn find(&self, pattern: &[String], start: usize, eof: bool) -> Option<SequenceMatch> {
        if pattern.is_empty() {
            return Some(SequenceMatch { start, length: 0 });
        }
        if pattern.len() > self.line_keys.len() {
            return None;
        }
        let pattern_keys = self.pattern_keys(pattern)?;
        let last_start = self.line_keys.len() - pattern_keys.len();
        if eof {
            return self
                .window_matches(last_start, &pattern_keys)
                .then_some(SequenceMatch {
                    start: last_start,
                    length: pattern_keys.len(),
                });
        }
        if start > last_start {
            return None;
        }
        self.find_from_anchor(&pattern_keys, start, last_start)
    }
    fn pattern_keys(&self, pattern: &[String]) -> Option<Vec<usize>> {
        pattern
            .iter()
            .map(|line| {
                let key = self.mode.key(line);
                self.key_ids.get(key.as_ref()).copied()
            })
            .collect()
    }
    fn find_from_anchor(
        &self,
        pattern_keys: &[usize],
        start: usize,
        last_start: usize,
    ) -> Option<SequenceMatch> {
        let anchor_offset = self.rarest_pattern_offset(pattern_keys)?;
        let anchor_key = *pattern_keys.get(anchor_offset)?;
        let anchor_postings = self.postings.get(anchor_key)?;
        for anchor_index in anchor_postings {
            if *anchor_index < anchor_offset {
                continue;
            }
            let candidate_start = *anchor_index - anchor_offset;
            if candidate_start < start || candidate_start > last_start {
                continue;
            }
            if self.window_matches(candidate_start, pattern_keys) {
                return Some(SequenceMatch {
                    start: candidate_start,
                    length: pattern_keys.len(),
                });
            }
        }
        None
    }
    fn rarest_pattern_offset(&self, pattern_keys: &[usize]) -> Option<usize> {
        let mut rarest = None;
        for (offset, key_id) in pattern_keys.iter().copied().enumerate() {
            let posting_count = self.postings.get(key_id)?.len();
            if rarest.is_none_or(|(_, rarest_count)| posting_count < rarest_count) {
                rarest = Some((offset, posting_count));
            }
        }
        rarest.map(|(offset, _)| offset)
    }
    fn window_matches(&self, start: usize, pattern_keys: &[usize]) -> bool {
        let Some(end) = start.checked_add(pattern_keys.len()) else {
            return false;
        };
        self.line_keys
            .get(start..end)
            .is_some_and(|window| window == pattern_keys)
    }
}
#[cfg(test)]
mod tests;
