use crate::config::Settings;
use std::{
    env,
    path::{Path, PathBuf},
};
const CONFIG_PATH_ERROR: &str = "configured apply-patch executable does not exist";
const PATH_CANDIDATES: &[&str] = &[
    "apply-patch.exe",
    "apply_patch.exe",
    "apply-patch",
    "apply_patch",
    "codex.exe",
    "codex",
];
const CODEX_APPLY_PATCH_ARG: &str = "--codex-run-as-apply-patch";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPatchCommand {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}
impl ApplyPatchCommand {
    #[must_use]
    pub const fn new(executable: PathBuf, arguments: Vec<String>) -> Self {
        Self {
            executable,
            arguments,
        }
    }
}
pub fn resolve(settings: &Settings) -> anyhow::Result<ApplyPatchCommand> {
    if let Some(executable) = settings.executable_path.as_ref() {
        anyhow::ensure!(
            executable.exists(),
            "{CONFIG_PATH_ERROR}: {}",
            executable.display()
        );
        return Ok(ApplyPatchCommand::new(
            executable.clone(),
            settings.arguments.clone(),
        ));
    }
    if let Some(executable) = find_in_path(PATH_CANDIDATES) {
        return Ok(command_from_path(executable));
    }
    anyhow::bail!("apply-patch or codex executable was not found in config or PATH");
}
fn find_in_path(names: &[&str]) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    let directories: Vec<PathBuf> = env::split_paths(&paths).collect();
    names
        .iter()
        .flat_map(|name| {
            directories
                .iter()
                .map(move |directory| candidate_path(directory, name))
        })
        .find(|candidate| candidate.is_file())
}
fn command_from_path(executable: PathBuf) -> ApplyPatchCommand {
    let is_codex = executable
        .file_stem()
        .is_some_and(|stem| stem.eq_ignore_ascii_case("codex"));
    let arguments = if is_codex {
        vec![String::from(CODEX_APPLY_PATCH_ARG)]
    } else {
        Vec::new()
    };
    ApplyPatchCommand::new(executable, arguments)
}
fn candidate_path(directory: &Path, name: &str) -> PathBuf {
    directory.join(name)
}
#[cfg(test)]
#[expect(
    clippy::inline_modules,
    reason = "unit tests stay next to private helpers"
)]
mod tests {
    use super::{CODEX_APPLY_PATCH_ARG, candidate_path, command_from_path};
    use std::path::PathBuf;
    #[test]
    fn candidates_include_unmodified_name() {
        let candidate = candidate_path("bin".as_ref(), "apply-patch");
        assert_eq!(candidate, PathBuf::from("bin/apply-patch"));
    }
    #[test]
    fn codex_from_path_gets_apply_patch_argument() {
        let command = command_from_path("bin/codex.exe".into());
        assert_eq!(command.arguments, vec![String::from(CODEX_APPLY_PATCH_ARG)]);
    }
}
