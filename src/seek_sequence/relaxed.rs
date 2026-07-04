use super::{MatchMode, SequenceMatch};
use alloc::borrow::Cow;
pub(super) fn find_relaxed(
    lines: &[&str],
    pattern: &[String],
    start: usize,
    eof: bool,
    mode: MatchMode,
) -> Option<SequenceMatch> {
    if pattern.is_empty() {
        return Some(SequenceMatch { start, length: 0 });
    }
    let pattern_keys = collect_pattern_keys(pattern, mode);
    if pattern_keys.is_empty() {
        return None;
    }
    let line_keys = collect_line_keys(lines, mode);
    if pattern_keys.len() > line_keys.len() {
        return None;
    }
    let last_start = line_keys.len() - pattern_keys.len();
    if eof {
        return window_matches(&line_keys, last_start, &pattern_keys);
    }
    if line_keys
        .get(last_start)
        .is_none_or(|line_key| start > line_key.line_index)
    {
        return None;
    }
    find_from_anchor(&line_keys, &pattern_keys, start, last_start)
}
fn collect_pattern_keys(pattern: &[String], mode: MatchMode) -> Vec<Cow<'_, str>> {
    pattern
        .iter()
        .filter_map(|line| {
            let line_key = mode.key(line);
            (!mode.skips_empty_lines() || !line_key.is_empty()).then_some(line_key)
        })
        .collect()
}
struct LineKey<'line> {
    line_index: usize,
    value: Cow<'line, str>,
}
fn collect_line_keys<'line>(lines: &'line [&'line str], mode: MatchMode) -> Vec<LineKey<'line>> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, line)| {
            let line_key = mode.key(line);
            (!mode.skips_empty_lines() || !line_key.is_empty()).then_some(LineKey {
                line_index,
                value: line_key,
            })
        })
        .collect()
}
fn find_from_anchor(
    line_keys: &[LineKey<'_>],
    pattern_keys: &[Cow<'_, str>],
    start: usize,
    last_entry_start: usize,
) -> Option<SequenceMatch> {
    let postings = collect_postings(line_keys, pattern_keys);
    let anchor_offset = rarest_pattern_offset(&postings)?;
    let anchor_postings = postings.get(anchor_offset)?;
    for anchor_entry_index in anchor_postings {
        if *anchor_entry_index < anchor_offset {
            continue;
        }
        let candidate_entry_start = *anchor_entry_index - anchor_offset;
        if candidate_entry_start > last_entry_start
            || line_keys
                .get(candidate_entry_start)
                .is_none_or(|line_key| line_key.line_index < start)
        {
            continue;
        }
        if let found @ Some(_) = window_matches(line_keys, candidate_entry_start, pattern_keys) {
            return found;
        }
    }
    None
}
fn collect_postings(line_keys: &[LineKey<'_>], pattern_keys: &[Cow<'_, str>]) -> Vec<Vec<usize>> {
    let mut postings = vec![Vec::new(); pattern_keys.len()];
    for (entry_index, line_key) in line_keys.iter().enumerate() {
        for (offset, pattern_key) in pattern_keys.iter().enumerate() {
            if line_key.value.as_ref() == pattern_key.as_ref()
                && let Some(posting) = postings.get_mut(offset)
            {
                posting.push(entry_index);
            }
        }
    }
    postings
}
fn rarest_pattern_offset(postings: &[Vec<usize>]) -> Option<usize> {
    let mut rarest = None;
    for (offset, posting) in postings.iter().enumerate() {
        if posting.is_empty() {
            return None;
        }
        if rarest.is_none_or(|(_, rarest_count)| posting.len() < rarest_count) {
            rarest = Some((offset, posting.len()));
        }
    }
    rarest.map(|(offset, _)| offset)
}
fn window_matches(
    line_keys: &[LineKey<'_>],
    entry_start: usize,
    pattern_keys: &[Cow<'_, str>],
) -> Option<SequenceMatch> {
    let entry_end = entry_start.checked_add(pattern_keys.len())?;
    let window = line_keys.get(entry_start..entry_end)?;
    if !window
        .iter()
        .zip(pattern_keys)
        .all(|(line_key, pattern_key)| line_key.value.as_ref() == pattern_key.as_ref())
    {
        return None;
    }
    let first_line = window.first()?;
    let last_line = window.last()?;
    let length = last_line.line_index.checked_sub(first_line.line_index)? + 1;
    Some(SequenceMatch {
        start: first_line.line_index,
        length,
    })
}
