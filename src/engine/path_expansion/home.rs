use super::PathExpansionError;
use std::path::PathBuf;
pub(super) fn needs_tilde_expansion(path: &str) -> bool {
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
pub(super) fn tilde_input<HomeLookup>(
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
    let Some(suffix) = path.get(1..) else {
        return Err(PathExpansionError::InvalidBoundary {
            path: path.to_owned(),
        });
    };
    expanded.push_str(suffix);
    Ok((expanded, None))
}
