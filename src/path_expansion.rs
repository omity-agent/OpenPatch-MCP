mod home;
#[cfg(test)]
mod tests;
mod variables;
use home::expand_tilde;
use std::{ffi::OsString, path::PathBuf};
use variables::expand_variables;
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
    if !needs_expansion(path) {
        return Ok(PathBuf::from(path));
    }
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
    let tilde_expanded = expand_tilde(path, home_lookup)?;
    if !has_variable_marker(&tilde_expanded) {
        return Ok(tilde_expanded);
    }
    expand_variables(&tilde_expanded, path, env_lookup)
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
