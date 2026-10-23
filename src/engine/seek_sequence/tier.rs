use super::{MatchMode, SequenceMatch};
use alloc::borrow::Cow;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use smallvec::SmallVec;
type Posting = SmallVec<[usize; 1]>;
type PatternKeys = SmallVec<[usize; 8]>;
pub(super) struct SearchTier<'text> {
    mode: MatchMode,
    entries: Vec<LineEntry>,
    postings: IndexMap<Cow<'text, str>, Posting, FxBuildHasher>,
}
struct LineEntry {
    line_index: usize,
    key_id: usize,
}
impl<'text> SearchTier<'text> {
    pub(super) fn new(lines: &[&'text str], mode: MatchMode) -> Self {
        let mut tier = Self {
            mode,
            entries: Vec::with_capacity(lines.len()),
            postings: IndexMap::with_capacity_and_hasher(lines.len(), FxBuildHasher),
        };
        for (index, line) in lines.iter().enumerate() {
            tier.push_line(index, line);
        }
        tier
    }
    pub(super) fn find(
        &self,
        pattern: &[String],
        start: usize,
        eof: bool,
    ) -> Option<SequenceMatch> {
        if pattern.is_empty() {
            return Some(SequenceMatch { start, length: 0 });
        }
        let pattern_keys = self.pattern_keys(pattern)?;
        if pattern_keys.is_empty() || pattern_keys.len() > self.entries.len() {
            return None;
        }
        let last_entry_start = self.entries.len() - pattern_keys.len();
        if eof {
            return self.window_matches(last_entry_start, &pattern_keys);
        }
        if self
            .entries
            .get(last_entry_start)
            .is_none_or(|entry| start > entry.line_index)
        {
            return None;
        }
        self.find_from_anchor(&pattern_keys, start, last_entry_start)
    }
    fn push_line(&mut self, index: usize, line: &'text str) {
        let key = self.mode.key(line);
        if self.mode.skips_empty_lines() && key.is_empty() {
            return;
        }
        let entry = self.postings.entry(key);
        let key_id = entry.index();
        let entry_index = self.entries.len();
        entry.or_default().push(entry_index);
        self.entries.push(LineEntry {
            line_index: index,
            key_id,
        });
    }
    fn pattern_keys(&self, pattern: &[String]) -> Option<PatternKeys> {
        let mut pattern_keys = SmallVec::new();
        for line in pattern {
            let key = self.mode.key(line);
            if self.mode.skips_empty_lines() && key.is_empty() {
                continue;
            }
            pattern_keys.push(self.postings.get_index_of(key.as_ref())?);
        }
        Some(pattern_keys)
    }
    fn find_from_anchor(
        &self,
        pattern_keys: &[usize],
        start: usize,
        last_entry_start: usize,
    ) -> Option<SequenceMatch> {
        let anchor_offset = self.rarest_pattern_offset(pattern_keys)?;
        let anchor_key = *pattern_keys.get(anchor_offset)?;
        let (_, anchor_postings) = self.postings.get_index(anchor_key)?;
        let lower = self
            .entries
            .partition_point(|entry| entry.line_index < start)
            .checked_add(anchor_offset)?;
        let upper = last_entry_start + anchor_offset;
        let first = anchor_postings.partition_point(|index| *index < lower);
        for anchor_entry_index in anchor_postings
            .iter()
            .skip(first)
            .take_while(|index| **index <= upper)
        {
            let candidate_entry_start = *anchor_entry_index - anchor_offset;
            if let found @ Some(_) = self.window_matches(candidate_entry_start, pattern_keys) {
                return found;
            }
        }
        None
    }
    fn rarest_pattern_offset(&self, pattern_keys: &[usize]) -> Option<usize> {
        let mut rarest = None;
        for (offset, key_id) in pattern_keys.iter().copied().enumerate() {
            let (_, posting) = self.postings.get_index(key_id)?;
            let posting_count = posting.len();
            if rarest.is_none_or(|(_, rarest_count)| posting_count < rarest_count) {
                rarest = Some((offset, posting_count));
            }
        }
        rarest.map(|(offset, _)| offset)
    }
    fn window_matches(&self, entry_start: usize, pattern_keys: &[usize]) -> Option<SequenceMatch> {
        let entry_end = entry_start.checked_add(pattern_keys.len())?;
        let window = self.entries.get(entry_start..entry_end)?;
        if !window
            .iter()
            .zip(pattern_keys)
            .all(|(entry, pattern_key)| entry.key_id == *pattern_key)
        {
            return None;
        }
        let first = window.first()?;
        let last = window.last()?;
        Some(SequenceMatch {
            start: first.line_index,
            length: last.line_index.checked_sub(first.line_index)? + 1,
        })
    }
}
