use std::collections::{HashMap, hash_map::Entry};
pub(crate) struct LineSearchIndex {
    exact: SearchTier,
    trim_end: SearchTier,
    trim: SearchTier,
    normalized: SearchTier,
}
impl LineSearchIndex {
    #[must_use]
    pub(crate) fn new(lines: &[String]) -> Self {
        Self {
            exact: SearchTier::new(lines, MatchMode::Exact),
            trim_end: SearchTier::new(lines, MatchMode::TrimEnd),
            trim: SearchTier::new(lines, MatchMode::Trim),
            normalized: SearchTier::new(lines, MatchMode::Normalized),
        }
    }
    #[must_use]
    pub(crate) fn seek(&self, pattern: &[String], start: usize, eof: bool) -> Option<usize> {
        self.exact
            .find(pattern, start, eof)
            .or_else(|| self.trim_end.find(pattern, start, eof))
            .or_else(|| self.trim.find(pattern, start, eof))
            .or_else(|| self.normalized.find(pattern, start, eof))
    }
}
#[derive(Clone, Copy)]
enum MatchMode {
    Exact,
    TrimEnd,
    Trim,
    Normalized,
}
impl MatchMode {
    fn key(self, source: &str) -> String {
        match self {
            Self::Exact => source.to_owned(),
            Self::TrimEnd => source.trim_end().to_owned(),
            Self::Trim => source.trim().to_owned(),
            Self::Normalized => normalize(source),
        }
    }
}
struct SearchTier {
    mode: MatchMode,
    line_keys: Vec<usize>,
    key_ids: HashMap<String, usize>,
    postings: Vec<Vec<usize>>,
}
impl SearchTier {
    fn new(lines: &[String], mode: MatchMode) -> Self {
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
    fn push_line(&mut self, index: usize, line: &str) {
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
    fn find(&self, pattern: &[String], start: usize, eof: bool) -> Option<usize> {
        if pattern.is_empty() {
            return Some(start);
        }
        if pattern.len() > self.line_keys.len() {
            return None;
        }
        let pattern_keys = self.pattern_keys(pattern)?;
        let last_start = self.line_keys.len() - pattern_keys.len();
        if eof {
            return self
                .window_matches(last_start, &pattern_keys)
                .then_some(last_start);
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
                self.key_ids.get(&key).copied()
            })
            .collect()
    }
    fn find_from_anchor(
        &self,
        pattern_keys: &[usize],
        start: usize,
        last_start: usize,
    ) -> Option<usize> {
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
                return Some(candidate_start);
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
fn normalize(source: &str) -> String {
    source
        .trim()
        .chars()
        .map(|character| match character {
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => '-',
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}'
            | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{205F}'
            | '\u{3000}' => ' ',
            other => other,
        })
        .collect()
}
#[cfg(test)]
mod tests;
