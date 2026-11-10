use super::map_characters;
use alloc::borrow::Cow;
use icu_properties::{CodePointMapData, props::GeneralCategory};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization as _;
use unicode_security::{mixed_script::AugmentedScriptSet, skeleton};
struct Representative {
    character: char,
    category: GeneralCategory,
    skeleton: String,
}
static LATIN_LETTERS: LazyLock<Vec<Representative>> = LazyLock::new(|| {
    let categories = CodePointMapData::<GeneralCategory>::new();
    ('A'..='Z')
        .chain('a'..='z')
        .map(|character| Representative {
            character,
            category: categories.get(character),
            skeleton: character_skeleton(character),
        })
        .collect()
});
pub(super) fn cross_script(source: &str) -> Cow<'_, str> {
    if source.is_ascii() {
        return Cow::Borrowed(source);
    }
    let categories = CodePointMapData::<GeneralCategory>::new();
    map_characters(source, |character| {
        if character.is_ascii() {
            return Some(character);
        }
        let category = categories.get(character);
        let prototype = character_skeleton(character);
        let matching_representative = LATIN_LETTERS.iter().find(|representative| {
            representative.category == category
                && representative.skeleton == prototype
                && different_scripts(character, representative.character)
        });
        if let Some(representative) = matching_representative {
            return Some(representative.character);
        }
        let mut characters = prototype.chars();
        if let Some(candidate) = characters.next()
            && characters.next().is_none()
            && categories.get(candidate) == category
            && different_scripts(character, candidate)
        {
            return Some(candidate);
        }
        Some(character)
    })
}
fn character_skeleton(character: char) -> String {
    let mut buffer = [0; 4];
    skeleton(character.encode_utf8(&mut buffer)).nfc().collect()
}
fn different_scripts(left: char, right: char) -> bool {
    let mut scripts = AugmentedScriptSet::for_char(left);
    scripts.intersect_with(AugmentedScriptSet::for_char(right));
    scripts.is_empty()
}
