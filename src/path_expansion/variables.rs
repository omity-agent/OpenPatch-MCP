use crate::path_expansion::{PathExpansionError, VariableValue};
pub(super) fn expand_variables<EnvLookup>(
    path: &str,
    original_path: &str,
    env_lookup: EnvLookup,
) -> Result<String, PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
{
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < path.len() {
        let path_tail = slice_from(path, cursor);
        let Some((offset, marker)) = path_tail
            .char_indices()
            .find(|&(_, character)| character == '$' || character == '%')
        else {
            output.push_str(path_tail);
            break;
        };
        let marker_index = cursor + offset;
        output.push_str(slice_between(path, cursor, marker_index));
        let next_index = marker_index + marker.len_utf8();
        cursor = if marker == '$' {
            expand_unix_variable(path, original_path, next_index, &env_lookup, &mut output)?
        } else {
            expand_windows_variable(path, original_path, next_index, &env_lookup, &mut output)?
        };
    }
    Ok(output)
}
fn expand_unix_variable<EnvLookup>(
    path: &str,
    original_path: &str,
    next_index: usize,
    env_lookup: &EnvLookup,
    output: &mut String,
) -> Result<usize, PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
{
    if slice_from(path, next_index).starts_with('{') {
        let name_start = next_index + 1;
        let Some(close_offset) = slice_from(path, name_start).find('}') else {
            return Err(PathExpansionError::UnterminatedBracedVariable {
                path: original_path.to_owned(),
            });
        };
        let name_end = name_start + close_offset;
        let name = slice_between(path, name_start, name_end);
        push_variable_value(original_path, name, env_lookup, output)?;
        return Ok(name_end + 1);
    }
    let name_end = consume_unix_variable_name(path, next_index);
    if name_end == next_index {
        output.push('$');
        return Ok(next_index);
    }
    let name = slice_between(path, next_index, name_end);
    push_variable_value(original_path, name, env_lookup, output)?;
    Ok(name_end)
}
fn expand_windows_variable<EnvLookup>(
    path: &str,
    original_path: &str,
    next_index: usize,
    env_lookup: &EnvLookup,
    output: &mut String,
) -> Result<usize, PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
{
    let Some(close_offset) = slice_from(path, next_index).find('%') else {
        output.push('%');
        return Ok(next_index);
    };
    let name_end = next_index + close_offset;
    let name = slice_between(path, next_index, name_end);
    if !is_windows_variable_name(name) {
        output.push('%');
        return Ok(next_index);
    }
    push_variable_value(original_path, name, env_lookup, output)?;
    Ok(name_end + 1)
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
    if name.is_empty() {
        return Err(PathExpansionError::EmptyVariable {
            path: path.to_owned(),
        });
    }
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
fn consume_unix_variable_name(path: &str, start_index: usize) -> usize {
    slice_from(path, start_index)
        .char_indices()
        .take_while(|&(_, character)| character.is_ascii_alphanumeric() || character == '_')
        .last()
        .map_or(start_index, |(offset, character)| {
            start_index + offset + character.len_utf8()
        })
}
fn slice_from(value: &str, start: usize) -> &str {
    let Some(slice) = value.get(start..) else {
        panic!("path expansion cursor is always on a UTF-8 boundary");
    };
    slice
}
fn slice_between(value: &str, start: usize, end: usize) -> &str {
    let Some(slice) = value.get(start..end) else {
        panic!("path expansion range is always on UTF-8 boundaries");
    };
    slice
}
fn is_windows_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| !character.is_whitespace() && character != '/' && character != '\\')
}
