use super::{collect_characters, map_characters};
use alloc::borrow::Cow;
use icu_properties::{CodePointSetData, props::DefaultIgnorableCodePoint};
use unicode_normalization::UnicodeNormalization as _;
pub(super) fn ignore_invisible(source: &str) -> Cow<'_, str> {
    if source.is_ascii() {
        return Cow::Borrowed(source);
    }
    let invisible = CodePointSetData::new::<DefaultIgnorableCodePoint>();
    map_characters(source, |character| {
        (!invisible.contains(character)).then_some(character)
    })
}
pub(super) fn unicode_compatibility(source: &str) -> Cow<'_, str> {
    if source.is_ascii() {
        return Cow::Borrowed(source);
    }
    collect_characters(source, source.nfkc())
}
pub(super) fn punctuation(source: &str) -> Cow<'_, str> {
    map_characters(source, |character| {
        Some(match character {
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => '-',
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            other => other,
        })
    })
}
