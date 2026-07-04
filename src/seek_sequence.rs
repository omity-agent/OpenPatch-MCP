pub(crate) fn seek_sequence(
    lines: &[String],
    pattern: &[String],
    start: usize,
    eof: bool,
) -> Option<usize> {
    if pattern.is_empty() {
        return Some(start);
    }
    if pattern.len() > lines.len() {
        return None;
    }
    let search_start = if eof && lines.len() >= pattern.len() {
        lines.len() - pattern.len()
    } else {
        start
    };
    let search_count = lines
        .len()
        .saturating_sub(pattern.len())
        .saturating_sub(search_start)
        + 1;
    find_window(
        lines,
        pattern,
        search_start,
        search_count,
        |line, pattern_line| line == pattern_line,
    )
    .or_else(|| {
        find_window(
            lines,
            pattern,
            search_start,
            search_count,
            |line, pattern_line| line.trim_end() == pattern_line.trim_end(),
        )
    })
    .or_else(|| {
        find_window(
            lines,
            pattern,
            search_start,
            search_count,
            |line, pattern_line| line.trim() == pattern_line.trim(),
        )
    })
    .or_else(|| {
        find_window(
            lines,
            pattern,
            search_start,
            search_count,
            |line, pattern_line| normalize(line) == normalize(pattern_line),
        )
    })
}
fn find_window(
    lines: &[String],
    pattern: &[String],
    search_start: usize,
    search_count: usize,
    predicate: impl Fn(&str, &str) -> bool,
) -> Option<usize> {
    lines
        .windows(pattern.len())
        .enumerate()
        .skip(search_start)
        .take(search_count)
        .find_map(|(index, window)| {
            window
                .iter()
                .zip(pattern)
                .all(|(line, pattern_line)| predicate(line, pattern_line))
                .then_some(index)
        })
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
