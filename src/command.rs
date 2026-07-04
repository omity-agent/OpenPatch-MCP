use crate::patch::{ApplyResult, apply_patch_text};
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
        format!(
            "exit_code: {}\nstdout:\n{}\nstderr:\n{}",
            self.status.map_or_else(
                || String::from("terminated by signal"),
                |status| status.to_string()
            ),
            self.stdout,
            self.stderr
        )
    }
}
pub fn normalize_cwd(cwd: Option<String>) -> anyhow::Result<PathBuf> {
    let resolved_cwd = cwd.map_or_else(std::env::current_dir, |path| Ok(PathBuf::from(path)))?;
    anyhow::ensure!(
        resolved_cwd.is_dir(),
        "cwd is not a directory: {}",
        resolved_cwd.display()
    );
    Ok(resolved_cwd)
}
#[cfg(test)]
mod tests {
    use super::{PatchExecution, PatchOutput, PatchRunner, normalize_cwd};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    #[test]
    fn success_requires_zero_status() {
        assert!(
            PatchOutput {
                status: Some(0_i32),
                stdout: String::new(),
                stderr: String::new()
            }
            .succeeded()
        );
    }
    #[test]
    fn missing_cwd_uses_current_dir() {
        let result = normalize_cwd(None);
        assert!(result.as_ref().is_ok_and(|path| path.is_dir()));
    }
    #[test]
    fn multiline_patch_is_applied_without_executable() {
        let directory = unique_temp_directory().unwrap();
        let target_path = directory.join("target.txt");
        fs::write(&target_path, "old\n").unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Update File: target.txt",
            "@@",
            "-old",
            "+new",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let output = PatchRunner::apply(PatchExecution {
            patch: &patch,
            cwd: &directory,
        });
        assert!(output.succeeded(), "{}", output.render());
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "new\n");
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn failed_update_does_not_stop_following_files() {
        let directory = unique_temp_directory().unwrap();
        let first_path = directory.join("a.txt");
        let second_path = directory.join("b.txt");
        let third_path = directory.join("c.txt");
        fs::write(&first_path, "old\n").unwrap();
        fs::write(&second_path, "kept\n").unwrap();
        fs::write(&third_path, "old\n").unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Update File: a.txt",
            "@@",
            "-old",
            "+new",
            "*** Update File: b.txt",
            "@@",
            "-missing",
            "+changed",
            "*** Update File: c.txt",
            "@@",
            "-old",
            "+new",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let output = PatchRunner::apply(PatchExecution {
            patch: &patch,
            cwd: &directory,
        });
        assert!(!output.succeeded());
        assert_eq!(fs::read_to_string(&first_path).unwrap(), "new\n");
        assert_eq!(fs::read_to_string(&second_path).unwrap(), "kept\n");
        assert_eq!(fs::read_to_string(&third_path).unwrap(), "new\n");
        assert!(output.stdout.contains("M a.txt"));
        assert!(output.stdout.contains("M c.txt"));
        assert!(output.stderr.contains("Failed to find expected lines"));
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn delete_missing_file_reports_delete_context() {
        let directory = unique_temp_directory().unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Delete File: missing.txt",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let output = PatchRunner::apply(PatchExecution {
            patch: &patch,
            cwd: &directory,
        });
        assert!(!output.succeeded());
        assert!(output.stderr.contains("Failed to delete file"));
        assert!(!output.stderr.contains("Failed to inspect file"));
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn delete_directory_reports_reference_style_context() {
        let directory = unique_temp_directory().unwrap();
        fs::create_dir_all(directory.join("target")).unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Delete File: target",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let output = PatchRunner::apply(PatchExecution {
            patch: &patch,
            cwd: &directory,
        });
        assert!(!output.succeeded());
        assert!(output.stderr.contains("Failed to delete file"));
        assert!(output.stderr.contains("path is a directory"));
        fs::remove_dir_all(directory).unwrap();
    }
    fn unique_temp_directory() -> anyhow::Result<PathBuf> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory =
            std::env::temp_dir().join(format!("apply-patch-mcp-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }
}
