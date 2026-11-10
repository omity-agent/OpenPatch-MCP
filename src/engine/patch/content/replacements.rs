pub(super) type Replacement<'new> = (usize, usize, &'new [String]);
pub(super) fn apply_replacements(
    original_contents: &str,
    lines: &[&str],
    replacements: &[Replacement<'_>],
) -> String {
    let mut result = String::with_capacity(original_contents.len());
    let mut source_index = 0;
    let mut line_written = false;
    for replacement in replacements {
        let start_index = replacement.0;
        let old_length = replacement.1;
        assert!(
            source_index <= start_index,
            "replacement ranges must be ordered and in bounds"
        );
        append_lines(
            &mut result,
            lines[source_index..start_index]
                .iter()
                .copied()
                .chain(replacement.2.iter().map(String::as_str)),
            &mut line_written,
        );
        let Some(next_index) = start_index.checked_add(old_length) else {
            panic!("replacement ranges must be ordered and in bounds");
        };
        source_index = next_index;
    }
    append_lines(
        &mut result,
        lines[source_index..].iter().copied(),
        &mut line_written,
    );
    if line_written && original_contents.ends_with('\n') {
        result.push('\n');
    }
    result
}
fn append_lines<'line>(
    result: &mut String,
    lines: impl Iterator<Item = &'line str>,
    line_written: &mut bool,
) {
    for line in lines {
        if *line_written {
            result.push('\n');
        }
        result.push_str(line);
        *line_written = true;
    }
}
