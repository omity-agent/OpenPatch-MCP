use super::map_characters;
use alloc::borrow::Cow;
use icu_properties::{CodePointMapData, props::GeneralCategory};
pub(super) fn unicode_spaces(source: &str) -> Cow<'_, str> {
    if source.is_ascii() {
        return Cow::Borrowed(source);
    }
    let categories = CodePointMapData::<GeneralCategory>::new();
    map_characters(source, |character| {
        Some(
            if categories.get(character) == GeneralCategory::SpaceSeparator {
                ' '
            } else {
                character
            },
        )
    })
}
pub(super) fn tabs(source: &str) -> Cow<'_, str> {
    map_characters(source, |character| {
        Some(if character == '\t' { ' ' } else { character })
    })
}
pub(super) fn collapse_spaces(source: &str) -> Cow<'_, str> {
    let mut previous_was_space = false;
    map_characters(source, |character| {
        let keep = character != ' ' || !previous_was_space;
        previous_was_space = character == ' ';
        keep.then_some(character)
    })
}
