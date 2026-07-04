use alloc::borrow::Cow;
pub(super) fn normalize(source: &str) -> Cow<'_, str> {
    let trimmed = source.trim();
    let mut normalized: Option<String> = None;
    for (index, character) in trimmed.char_indices() {
        let replacement = normalize_character(character);
        if let Some(output) = normalized.as_mut() {
            output.push(replacement);
        } else if replacement != character {
            let mut output = String::with_capacity(trimmed.len());
            let Some(prefix) = trimmed.get(..index) else {
                panic!("char index must be a string boundary");
            };
            output.push_str(prefix);
            output.push(replacement);
            normalized = Some(output);
        }
    }
    normalized.map_or(Cow::Borrowed(trimmed), Cow::Owned)
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
