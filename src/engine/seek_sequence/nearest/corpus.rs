use crate::seek_sequence::{SequenceMatch, normalize::collapse_spaces};
pub(super) struct NormalizedCorpus {
    chars: Vec<char>,
    line_starts: Vec<(usize, usize)>,
}
impl NormalizedCorpus {
    pub(super) fn new(lines: &[&str]) -> Self {
        let mut chars = Vec::new();
        let mut line_starts = Vec::new();
        for (line_index, line) in lines.iter().enumerate() {
            let normalized = collapse_spaces(line);
            if normalized.is_empty() {
                continue;
            }
            if !chars.is_empty() {
                chars.push('\n');
            }
            line_starts.push((chars.len(), line_index));
            chars.extend(normalized.chars());
        }
        Self { chars, line_starts }
    }
    pub(super) const fn chars(&self) -> &[char] {
        self.chars.as_slice()
    }
    pub(super) const fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
    pub(super) fn sequence(&self, start: usize, end: usize) -> Option<SequenceMatch> {
        let fragment = self.chars.get(start..end)?;
        let first_offset = fragment.iter().position(|character| *character != '\n')?;
        let last_offset = fragment.iter().rposition(|character| *character != '\n')?;
        let first_index = start.checked_add(first_offset)?;
        let last_index = start.checked_add(last_offset)?;
        let first_line = self.line(first_index)?;
        let last_line = self.line(last_index)?;
        let length = last_line.checked_sub(first_line)?.checked_add(1)?;
        Some(SequenceMatch {
            start: first_line,
            length,
        })
    }
    fn line(&self, character_index: usize) -> Option<usize> {
        let following = self
            .line_starts
            .partition_point(|&(start, _)| start <= character_index);
        self.line_starts
            .get(following.checked_sub(1)?)
            .map(|&(_, line)| line)
    }
}
