mod characters;
mod confusables;
mod whitespace;
use alloc::borrow::Cow;
type Transform = fn(&str) -> Cow<'_, str>;
enum Rule {
    Trim,
    Transform(Transform),
    IgnoreEmptyLines,
}
const RULES: &[Rule] = &[
    Rule::Trim,
    Rule::Transform(characters::ignore_invisible),
    Rule::Transform(whitespace::unicode_spaces),
    Rule::Transform(whitespace::tabs),
    Rule::Transform(characters::unicode_compatibility),
    Rule::Transform(characters::punctuation),
    Rule::IgnoreEmptyLines,
    Rule::Transform(whitespace::collapse_spaces),
    Rule::Transform(confusables::cross_script),
];
#[derive(Clone, Copy)]
pub(super) struct RuleChain(&'static [Rule]);
impl RuleChain {
    pub(super) fn prefixes() -> impl Iterator<Item = Self> {
        (0..=RULES.len()).map(|length| Self(RULES.split_at(length).0))
    }
    pub(super) const fn complete() -> Self {
        Self(RULES)
    }
    pub(super) fn key(self, source: &str) -> Option<Cow<'_, str>> {
        let mut key = Cow::Borrowed(source);
        let mut trim = false;
        let mut ignore_empty = false;
        for rule in self.0 {
            match *rule {
                Rule::Trim => trim = true,
                Rule::Transform(transform) => key = apply(key, transform),
                Rule::IgnoreEmptyLines => ignore_empty = true,
            }
        }
        if trim {
            key = apply(key, |text| Cow::Borrowed(text.trim()));
        }
        if ignore_empty && key.is_empty() {
            return None;
        }
        Some(key)
    }
}
fn apply(source: Cow<'_, str>, transform: Transform) -> Cow<'_, str> {
    match source {
        Cow::Borrowed(text) => transform(text),
        Cow::Owned(text) => match transform(&text) {
            Cow::Borrowed(borrowed) if core::ptr::eq(borrowed, text.as_str()) => Cow::Owned(text),
            Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
            Cow::Owned(transformed) => Cow::Owned(transformed),
        },
    }
}
fn map_characters(source: &str, mut transform: impl FnMut(char) -> Option<char>) -> Cow<'_, str> {
    let mut output: Option<String> = None;
    for (offset, character) in source.char_indices() {
        let replacement = transform(character);
        if output.is_none() && replacement != Some(character) {
            let (prefix, _) = source.split_at(offset);
            let mut buffer = String::with_capacity(source.len());
            buffer.push_str(prefix);
            output = Some(buffer);
        }
        if let Some(buffer) = output.as_mut()
            && let Some(value) = replacement
        {
            buffer.push(value);
        }
    }
    output.map_or(Cow::Borrowed(source), Cow::Owned)
}
fn collect_characters(source: &str, characters: impl Iterator<Item = char>) -> Cow<'_, str> {
    let output = characters.collect::<String>();
    if output == source {
        Cow::Borrowed(source)
    } else {
        Cow::Owned(output)
    }
}
