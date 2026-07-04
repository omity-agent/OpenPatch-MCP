use crate::path_expansion::PathExpansionError;
use std::path::PathBuf;
pub(super) fn expand_tilde<HomeLookup>(
    path: &str,
    home_lookup: HomeLookup,
) -> Result<String, PathExpansionError>
where
    HomeLookup: Fn() -> Option<PathBuf>,
{
    if path != "~" && !path.starts_with("~/") && !path.starts_with("~\\") {
        return Ok(path.to_owned());
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
    if path == "~" {
        Ok(home_string)
    } else {
        let mut characters = path.chars();
        let _tilde = characters.next();
        let remainder = characters.as_str();
        let mut output = String::with_capacity(home_string.len() + remainder.len());
        output.push_str(&home_string);
        output.push_str(remainder);
        Ok(output)
    }
}
