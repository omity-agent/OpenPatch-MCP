pub(super) type Replacement<'new> = (usize, usize, &'new [String]);
pub(super) fn apply_replacements(
    original_contents: &str,
    lines: &[&str],
    offsets: &[usize],
    replacements: &[Replacement<'_>],
) -> String {
    let mut result = String::with_capacity(original_contents.len());
    let mut source_index = 0;
    let mut line_written = false;
    let mut last_line_empty = false;
    for replacement in replacements {
        let start_index = replacement.0;
        let old_length = replacement.1;
        assert!(
            source_index <= start_index,
            "replacement ranges must be ordered and in bounds"
        );
        append_original_span(
            &mut result,
            &OriginalSpan {
                contents: original_contents,
                lines,
                offsets,
                start: source_index,
                end: start_index,
            },
            &mut line_written,
            &mut last_line_empty,
        );
        append_replacement_lines(
            &mut result,
            replacement.2,
            &mut line_written,
            &mut last_line_empty,
        );
        let Some(next_index) = start_index.checked_add(old_length) else {
            panic!("replacement ranges must be ordered and in bounds");
        };
        source_index = next_index.min(lines.len());
    }
    append_original_span(
        &mut result,
        &OriginalSpan {
            contents: original_contents,
            lines,
            offsets,
            start: source_index,
            end: lines.len(),
        },
        &mut line_written,
        &mut last_line_empty,
    );
    if line_written && !last_line_empty {
        result.push('\n');
    }
    result
}
struct OriginalSpan<'content, 'lines, 'offsets> {
    contents: &'content str,
    lines: &'lines [&'content str],
    offsets: &'offsets [usize],
    start: usize,
    end: usize,
}
fn append_original_span(
    result: &mut String,
    span: &OriginalSpan<'_, '_, '_>,
    line_written: &mut bool,
    last_line_empty: &mut bool,
) {
    if span.start == span.end {
        return;
    }
    assert!(
        span.start < span.end && span.end <= span.lines.len(),
        "replacement ranges must be ordered and in bounds"
    );
    if *line_written {
        result.push('\n');
    }
    let Some(byte_start) = span.offsets.get(span.start).copied() else {
        panic!("line start offset must exist");
    };
    let last_line_index = span.end - 1;
    let byte_end = original_span_byte_end(span, last_line_index);
    let Some(original_slice) = span.contents.get(byte_start..byte_end) else {
        panic!("line byte range must be valid");
    };
    result.push_str(original_slice);
    *line_written = true;
    *last_line_empty = byte_start == byte_end;
}
fn original_span_byte_end(span: &OriginalSpan<'_, '_, '_>, last_line_index: usize) -> usize {
    if let Some(next_offset) = span.offsets.get(last_line_index + 1).copied() {
        return next_offset.saturating_sub(1);
    }
    span.contents
        .len()
        .saturating_sub(usize::from(span.contents.ends_with('\n')))
}
fn append_replacement_lines(
    result: &mut String,
    lines: &[String],
    line_written: &mut bool,
    last_line_empty: &mut bool,
) {
    for line in lines {
        if *line_written {
            result.push('\n');
        }
        result.push_str(line);
        *line_written = true;
        *last_line_empty = line.is_empty();
    }
}
