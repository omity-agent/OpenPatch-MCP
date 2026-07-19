#[cfg(test)]
mod tests;
mod windows;
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
fn needs_tilde_expansion(path: &str) -> bool {
    path == "~" || path.starts_with("~/") || path.starts_with("~\\")
}
fn home_string<HomeLookup>(
    path: &str,
    home_lookup: HomeLookup,
) -> Result<Option<String>, PathExpansionError>
where
    HomeLookup: Fn() -> Option<PathBuf>,
{
    if !needs_tilde_expansion(path) {
        return Ok(None);
    }
    let Some(home) = home_lookup() else {
        return Err(PathExpansionError::MissingHome {
            path: path.to_owned(),
        });
    };
    let Ok(home_string) = home.into_os_string().into_string() else {
        return Err(PathExpansionError::NonUnicodeHome {
            path: path.to_owned(),
        });
    };
    Ok(Some(home_string))
}
fn tilde_input<HomeLookup>(
    path: &str,
    home_lookup: HomeLookup,
) -> Result<(String, Option<String>), PathExpansionError>
where
    HomeLookup: Fn() -> Option<PathBuf>,
{
    let home = home_string(path, home_lookup)?;
    if !path.starts_with("~\\") {
        return Ok((path.to_owned(), home));
    }
    let Some(home_string) = home.as_deref() else {
        return Err(PathExpansionError::MissingHome {
            path: path.to_owned(),
        });
    };
    let mut expanded = String::with_capacity(home_string.len() + path.len() - 1);
    expanded.push_str(home_string);
    expanded.push_str(path.get(1..).unwrap_or_default());
    Ok((expanded, None))
}
fn validate_unix_braces(path: &str) -> Result<(), PathExpansionError> {
    let mut cursor = 0;
    while let Some(open_offset) = path.get(cursor..).and_then(|tail| tail.find("${")) {
        let name_start = cursor + open_offset + 2;
        let Some(close_offset) = path.get(name_start..).and_then(|tail| tail.find('}')) else {
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
