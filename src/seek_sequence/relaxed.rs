use super::MatchMode;
use alloc::borrow::Cow;
pub(super) fn find_relaxed(
    lines: &[&str],
    pattern: &[String],
    start: usize,
    eof: bool,
    mode: MatchMode,
) -> Option<usize> {
    if pattern.is_empty() {
        return Some(start);
    }
    if pattern.len() > lines.len() {
        return None;
    }
    let pattern_keys = pattern
        .iter()
        .map(|line| mode.key(line))
        .collect::<Vec<_>>();
    let last_start = lines.len() - pattern_keys.len();
    if eof {
        return window_matches(lines, last_start, &pattern_keys, mode).then_some(last_start);
    }
    if start > last_start {
        return None;
    }
    find_from_anchor(lines, &pattern_keys, start, last_start, mode)
}
fn find_from_anchor(
    lines: &[&str],
    pattern_keys: &[Cow<'_, str>],
    start: usize,
    last_start: usize,
    mode: MatchMode,
) -> Option<usize> {
    let postings = collect_postings(lines, pattern_keys, mode);
    let anchor_offset = rarest_pattern_offset(&postings)?;
    let anchor_postings = postings.get(anchor_offset)?;
    for anchor_index in anchor_postings {
        if *anchor_index < anchor_offset {
            continue;
        }
        let candidate_start = *anchor_index - anchor_offset;
        if candidate_start < start || candidate_start > last_start {
            continue;
        }
        if window_matches(lines, candidate_start, pattern_keys, mode) {
            return Some(candidate_start);
        }
    }
    None
}
fn collect_postings(
    lines: &[&str],
    pattern_keys: &[Cow<'_, str>],
    mode: MatchMode,
) -> Vec<Vec<usize>> {
    let mut postings = vec![Vec::new(); pattern_keys.len()];
    for (index, line) in lines.iter().enumerate() {
        let line_key = mode.key(line);
        for (offset, pattern_key) in pattern_keys.iter().enumerate() {
            if line_key.as_ref() == pattern_key.as_ref()
                && let Some(posting) = postings.get_mut(offset)
            {
                posting.push(index);
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
    lines: &[&str],
    start: usize,
    pattern_keys: &[Cow<'_, str>],
    mode: MatchMode,
) -> bool {
    let Some(end) = start.checked_add(pattern_keys.len()) else {
        return false;
    };
    let Some(window) = lines.get(start..end) else {
        return false;
    };
    window
        .iter()
        .zip(pattern_keys)
        .all(|(line, pattern_key)| mode.key(line).as_ref() == pattern_key.as_ref())
}
