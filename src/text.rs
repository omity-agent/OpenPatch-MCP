pub(crate) fn line_count(contents: &str) -> usize {
    if contents.is_empty() {
        0
    } else {
        bytecount::count(contents.as_bytes(), b'\n') + usize::from(!contents.ends_with('\n'))
    }
}
pub(crate) fn character_count(contents: &str) -> usize {
    if contents.is_ascii() {
        contents.len()
    } else {
        bytecount::num_chars(contents.as_bytes())
    }
}
