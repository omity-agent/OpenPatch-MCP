use super::ranking::{Similarity, THRESHOLD};
use crate::seek_sequence::{SequenceMatch, normalize::collapse_spaces};
use core::ops::Range;
pub(super) struct FlattenedLines {
    chars: Vec<char>,
    entries: Vec<LineEntry>,
}
impl FlattenedLines {
    pub(super) fn new(lines: &[&str]) -> Self {
        let mut chars = Vec::new();
        let mut entries = Vec::with_capacity(lines.len());
        for (line_index, line) in lines.iter().enumerate() {
            let normalized = collapse_spaces(line);
            if normalized.is_empty() {
                continue;
            }
            if !chars.is_empty() {
                chars.push('\n');
            }
            let start = chars.len();
            chars.extend(normalized.chars());
            entries.push(LineEntry {
                line_index,
                start,
                end: chars.len(),
            });
        }
        Self { chars, entries }
    }
    pub(super) const fn entry_count(&self) -> usize {
        self.entries.len()
    }
    pub(super) fn candidate_range(&self, start: usize, pattern_length: usize) -> Range<usize> {
        let Some(start_entry) = self.entries.get(start) else {
            panic!("candidate start must reference a normalized line");
        };
        let remaining = self
            .entries
            .get(start..)
            .unwrap_or_else(|| panic!("candidate start must reference remaining lines"));
        let first = remaining.partition_point(|entry| {
            let length = entry.end - start_entry.start;
            length < pattern_length
                && Similarity::length_bound(pattern_length, length)
                    .compare(THRESHOLD)
                    .is_le()
        });
        let eligible = remaining
            .get(first..)
            .unwrap_or_else(|| panic!("eligible range must reference remaining lines"));
        let count = eligible.partition_point(|entry| {
            let length = entry.end - start_entry.start;
            Similarity::length_bound(pattern_length, length)
                .compare(THRESHOLD)
                .is_gt()
        });
        let range_start = start + first;
        range_start..range_start + count
    }
    pub(super) fn closest_length_index(
        &self,
        start: usize,
        pattern_length: usize,
        range: &Range<usize>,
    ) -> Option<usize> {
        let entries = self.entries.get(range.clone())?;
        let start_char = self.entries.get(start)?.start;
        let right_offset = entries.partition_point(|entry| entry.end - start_char < pattern_length);
        let right = range.start.checked_add(right_offset)?;
        let left = right.checked_sub(1).filter(|index| *index >= range.start);
        [left, (right < range.end).then_some(right)]
            .into_iter()
            .flatten()
            .min_by_key(|index| {
                self.entries.get(*index).map_or(usize::MAX, |entry| {
                    (entry.end - start_char).abs_diff(pattern_length)
                })
            })
    }
    pub(super) fn fragment(&self, start: usize, end: usize) -> &[char] {
        let Some(start_entry) = self.entries.get(start) else {
            panic!("fragment start must reference a normalized line");
        };
        let Some(end_entry) = self.entries.get(end) else {
            panic!("fragment end must reference a normalized line");
        };
        self.chars
            .get(start_entry.start..end_entry.end)
            .unwrap_or_else(|| panic!("fragment character range must be valid"))
    }
    pub(super) fn sequence(&self, start: usize, end: usize) -> SequenceMatch {
        let Some(first) = self.entries.get(start) else {
            panic!("sequence start must reference a normalized line");
        };
        let Some(last) = self.entries.get(end) else {
            panic!("sequence end must reference a normalized line");
        };
        SequenceMatch {
            start: first.line_index,
            length: last.line_index - first.line_index + 1,
        }
    }
}
struct LineEntry {
    line_index: usize,
    start: usize,
    end: usize,
}
