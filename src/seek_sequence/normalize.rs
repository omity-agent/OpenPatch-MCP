use alloc::borrow::Cow;
use unicode_normalization::UnicodeNormalization as _;
pub(super) fn normalize(source: &str) -> Cow<'_, str> {
    let trimmed = source.trim();
    let unicode_normalized = normalize_unicode(trimmed);
    let normalized = unicode_normalized.as_ref();
    if normalized.is_ascii() {
        return unicode_normalized;
    }
    let mut punctuation_normalized: Option<String> = None;
    for (index, character) in normalized.char_indices() {
        let replacement = normalize_character(character);
        if let Some(output) = punctuation_normalized.as_mut() {
            output.push(replacement);
        } else if replacement != character {
            let mut output = String::with_capacity(normalized.len());
            let Some(prefix) = normalized.get(..index) else {
                panic!("char index must be a string boundary");
            };
            output.push_str(prefix);
            output.push(replacement);
            punctuation_normalized = Some(output);
        }
    }
    punctuation_normalized.map_or(unicode_normalized, Cow::Owned)
}
fn normalize_unicode(source: &str) -> Cow<'_, str> {
    if source.is_ascii() {
        return Cow::Borrowed(source);
    }
    let normalized = source.nfkc().collect::<String>();
    if normalized == source {
        Cow::Borrowed(source)
    } else {
        Cow::Owned(normalized)
    }
}
pub(super) fn collapse_spaces(source: &str) -> Cow<'_, str> {
    let normalized = normalize(source);
    let mut collapsed: Option<String> = None;
    let mut previous_was_space = false;
    for (index, character) in normalized.char_indices() {
        let keep_character = character != ' ' || !previous_was_space;
        if let Some(output) = collapsed.as_mut() {
            if keep_character {
                output.push(character);
            }
        } else if keep_character {
            previous_was_space = character == ' ';
            continue;
        } else {
            let mut output = String::with_capacity(normalized.len());
            let Some(prefix) = normalized.get(..index) else {
                panic!("char index must be a string boundary");
            };
            output.push_str(prefix);
            collapsed = Some(output);
        }
        previous_was_space = character == ' ';
    }
    collapsed.map_or(normalized, Cow::Owned)
}
const fn normalize_character(character: char) -> char {
    match character {
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
        | '\u{2212}' => '-',
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
        '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}'
        | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{205F}'
        | '\u{3000}' => ' ',
        other => other,
    }
}
