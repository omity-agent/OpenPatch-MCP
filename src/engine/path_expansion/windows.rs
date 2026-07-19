use crate::path_expansion::{PathExpansionError, VariableValue};
pub(super) fn expand_windows_variables<EnvLookup>(
    path: &str,
    original_path: &str,
    env_lookup: EnvLookup,
) -> Result<String, PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
{
    let mut output = String::with_capacity(path.len());
    let mut cursor = 0;
    while let Some(marker_offset) = path.get(cursor..).and_then(|tail| tail.find('%')) {
        let marker_index = cursor + marker_offset;
        output.push_str(path_slice(path, cursor, marker_index));
        let name_start = marker_index + 1;
        let Some(close_offset) = path.get(name_start..).and_then(|tail| tail.find('%')) else {
            output.push('%');
            cursor = name_start;
            continue;
        };
        let name_end = name_start + close_offset;
        let name = path_slice(path, name_start, name_end);
        if is_windows_variable_name(name) {
            push_variable_value(original_path, name, &env_lookup, &mut output)?;
            cursor = name_end + 1;
        } else {
            output.push('%');
            cursor = name_start;
        }
    }
    output.push_str(path_slice(path, cursor, path.len()));
    Ok(output)
}
fn push_variable_value<EnvLookup>(
    path: &str,
    name: &str,
    env_lookup: &EnvLookup,
    output: &mut String,
) -> Result<(), PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
{
    match env_lookup(name) {
        VariableValue::Present(value) => output.push_str(&value),
        VariableValue::Missing => {
            return Err(PathExpansionError::MissingVariable {
                name: name.to_owned(),
                path: path.to_owned(),
            });
        }
        VariableValue::NonUnicode => {
            return Err(PathExpansionError::NonUnicodeVariable {
                name: name.to_owned(),
                path: path.to_owned(),
            });
        }
    }
    Ok(())
}
fn path_slice(value: &str, start: usize, end: usize) -> &str {
    let Some(slice) = value.get(start..end) else {
        panic!("path expansion range is always on UTF-8 boundaries");
    };
    slice
}
fn is_windows_variable_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.is_ascii() {
        return name
            .as_bytes()
            .iter()
            .all(|byte| !byte.is_ascii_whitespace() && !matches!(*byte, b'/' | b'\\'));
    }
    name.chars()
        .all(|character| !character.is_whitespace() && character != '/' && character != '\\')
}
