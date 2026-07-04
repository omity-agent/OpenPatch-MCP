use super::{MatchMode, SequenceMatch};
use alloc::borrow::Cow;
use rustc_hash::{FxBuildHasher, FxHashMap};
use smallvec::SmallVec;
use std::collections::hash_map::Entry;
type Posting = SmallVec<[usize; 1]>;
type PatternKeys = SmallVec<[usize; 8]>;
pub(super) struct SearchTier<'text> {
    mode: MatchMode,
    entries: Vec<LineEntry>,
    key_ids: FxHashMap<Cow<'text, str>, usize>,
    postings: Vec<Posting>,
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
            key_ids: FxHashMap::with_capacity_and_hasher(lines.len(), FxBuildHasher),
            postings: Vec::new(),
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
        let key_id = match self.key_ids.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let next_id = self.postings.len();
                entry.insert(next_id);
                self.postings.push(SmallVec::new());
                next_id
            }
        };
        let entry_index = self.entries.len();
        self.entries.push(LineEntry {
            line_index: index,
            key_id,
        });
        let Some(posting) = self.postings.get_mut(key_id) else {
            panic!("line key id must reference a posting list");
        };
        posting.push(entry_index);
    }
    fn pattern_keys(&self, pattern: &[String]) -> Option<PatternKeys> {
        let mut pattern_keys = SmallVec::new();
        for line in pattern {
            let key = self.mode.key(line);
            if self.mode.skips_empty_lines() && key.is_empty() {
                continue;
            }
            pattern_keys.push(self.key_ids.get(key.as_ref()).copied()?);
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
        let anchor_postings = self.postings.get(anchor_key)?;
        for anchor_entry_index in anchor_postings {
            if *anchor_entry_index < anchor_offset {
                continue;
            }
            let candidate_entry_start = *anchor_entry_index - anchor_offset;
            if candidate_entry_start > last_entry_start
                || self
                    .entries
                    .get(candidate_entry_start)
                    .is_none_or(|entry| entry.line_index < start)
            {
                continue;
            }
            if let found @ Some(_) = self.window_matches(candidate_entry_start, pattern_keys) {
                return found;
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
