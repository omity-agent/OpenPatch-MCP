mod home;
#[cfg(test)]
#[path = "../../test/engine/path_expansion.rs"]
mod tests;
mod windows;
use home::{needs_tilde_expansion, tilde_input};
use std::{ffi::OsString, path::PathBuf};
use windows::expand_windows_variables;
#[derive(Debug, Clone, PartialEq, Eq, thiserror :: Error)]
pub(crate) enum PathExpansionError {
    #[error("environment variable '{name}' is not set in path '{path}'")]
    MissingVariable { name: String, path: String },
    #[error("empty environment variable name in path '{path}'")]
    EmptyVariable { path: String },
    #[error("unterminated braced environment variable in path '{path}'")]
    UnterminatedBracedVariable { path: String },
    #[error("home directory is unavailable for path '{path}'")]
    MissingHome { path: String },
    #[error("home directory is not valid Unicode for path '{path}'")]
    NonUnicodeHome { path: String },
    #[error("path expansion reached an invalid UTF-8 boundary in path '{path}'")]
    InvalidBoundary { path: String },
    #[error("environment variable '{name}' is not valid Unicode in path '{path}'")]
    NonUnicodeVariable { name: String, path: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum VariableValue {
    Present(String),
    Missing,
    NonUnicode,
}
pub(crate) fn expand_path(path: &str) -> Result<PathBuf, PathExpansionError> {
    let expanded = expand_with(path, process_env, || {
        directories::BaseDirs::new().map(|directories| directories.home_dir().to_path_buf())
    })?;
    Ok(PathBuf::from(expanded))
}
fn expand_with<EnvLookup, HomeLookup>(
    path: &str,
    env_lookup: EnvLookup,
    home_lookup: HomeLookup,
) -> Result<String, PathExpansionError>
where
    EnvLookup: Fn(&str) -> VariableValue,
    HomeLookup: Fn() -> Option<PathBuf>,
{
    if !needs_expansion(path) {
        return Ok(path.to_owned());
    }
    validate_unix_braces(path)?;
    let (tilde_input, home) = tilde_input(path, home_lookup)?;
    let shell_expanded = shellexpand::full_with_context(
        &tilde_input,
        || home.as_deref(),
        |name| match env_lookup(name) {
            VariableValue::Present(value) => Ok(Some(value)),
            VariableValue::Missing => Err(PathExpansionError::MissingVariable {
                name: name.to_owned(),
                path: path.to_owned(),
            }),
            VariableValue::NonUnicode => Err(PathExpansionError::NonUnicodeVariable {
                name: name.to_owned(),
                path: path.to_owned(),
            }),
        },
    )
    .map_err(|error| error.cause)?;
    if shell_expanded.contains('%') {
        expand_windows_variables(&shell_expanded, path, env_lookup)
    } else {
        Ok(shell_expanded.into_owned())
    }
}
fn needs_expansion(path: &str) -> bool {
    needs_tilde_expansion(path) || has_variable_marker(path)
}
fn has_variable_marker(path: &str) -> bool {
    memchr::memchr2(b'$', b'%', path.as_bytes()).is_some()
}
fn validate_unix_braces(path: &str) -> Result<(), PathExpansionError> {
    let mut cursor = 0;
    loop {
        let Some(tail) = path.get(cursor..) else {
            return Err(PathExpansionError::InvalidBoundary {
                path: path.to_owned(),
            });
        };
        let Some(open_offset) = tail.find("${") else {
            break;
        };
        let name_start = cursor + open_offset + 2;
        let Some(name_tail) = path.get(name_start..) else {
            return Err(PathExpansionError::InvalidBoundary {
                path: path.to_owned(),
            });
        };
        let Some(close_offset) = name_tail.find('}') else {
            return Err(PathExpansionError::UnterminatedBracedVariable {
                path: path.to_owned(),
            });
        };
        if close_offset == 0 {
            return Err(PathExpansionError::EmptyVariable {
                path: path.to_owned(),
            });
        }
        cursor = name_start + close_offset + 1;
    }
    Ok(())
}
fn process_env(name: &str) -> VariableValue {
    std::env::var_os(name).map_or(VariableValue::Missing, VariableValue::from)
}
impl From<OsString> for VariableValue {
    fn from(raw_value: OsString) -> Self {
        raw_value
            .into_string()
            .map_or(Self::NonUnicode, Self::Present)
    }
}
