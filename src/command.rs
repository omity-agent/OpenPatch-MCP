use crate::{
    patch::{ApplyResult, apply_patch_text},
    path_expansion::expand_path,
};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, Default)]
pub struct PatchRunner;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Debug, Copy, Clone)]
pub struct PatchExecution<'request> {
    pub patch: &'request str,
    pub cwd: &'request Path,
}
impl PatchRunner {
    pub fn apply(request: PatchExecution<'_>) -> PatchOutput {
        let ApplyResult { stdout, stderr } = apply_patch_text(request.patch, request.cwd);
        let status = i32::from(!stderr.is_empty());
        PatchOutput {
            status: Some(status),
            stdout,
            stderr,
        }
    }
}
impl PatchOutput {
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }
    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::with_capacity(
            "exit_code: \nstdout:\n\nstderr:\n".len() + self.stdout.len() + self.stderr.len() + 11,
        );
        output.push_str("exit_code: ");
        if let Some(status) = self.status {
            let mut buffer = itoa::Buffer::new();
            output.push_str(buffer.format(status));
        } else {
            output.push_str("terminated by signal");
        }
        output.push_str("\nstdout:\n");
        output.push_str(&self.stdout);
        output.push_str("\nstderr:\n");
        output.push_str(&self.stderr);
        output
    }
}
pub fn normalize_cwd(cwd: Option<String>) -> anyhow::Result<PathBuf> {
    let resolved_cwd = match cwd {
        Some(path) => expand_path(&path).map_err(anyhow::Error::from)?,
        None => std::env::current_dir()?,
    };
    anyhow::ensure!(
        resolved_cwd.is_dir(),
        "cwd is not a directory: {}",
        resolved_cwd.display()
    );
    Ok(resolved_cwd)
}
#[cfg(test)]
mod tests;
