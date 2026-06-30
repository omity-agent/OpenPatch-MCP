use crate::locator::ApplyPatchCommand;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;
#[derive(Debug, Clone)]
pub struct PatchRunner {
    command: ApplyPatchCommand,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
impl PatchRunner {
    #[must_use]
    pub const fn new(command: ApplyPatchCommand) -> Self {
        Self { command }
    }
    pub async fn apply(&self, request: PatchExecution<'_>) -> anyhow::Result<PatchOutput> {
        let mut command = Command::new(&self.command.executable);
        command.args(&self.command.arguments);
        command.arg(request.patch);
        command.current_dir(request.cwd);
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let output = command.output().await?;
        Ok(PatchOutput {
            status: output.status.code(),
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
        })
    }
}
#[derive(Debug, Copy, Clone)]
pub struct PatchExecution<'request> {
    pub patch: &'request str,
    pub cwd: &'request Path,
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
#[expect(
    clippy::inline_modules,
    reason = "unit tests stay next to private helpers"
)]
mod tests {
    use super::{PatchExecution, PatchOutput, PatchRunner, normalize_cwd};
    use crate::locator::ApplyPatchCommand;
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
    #[derive(serde :: Deserialize)]
    struct CapturedInvocation {
        patch: String,
        stdin: String,
        extra_args: usize,
    }
    #[tokio::test]
    async fn multiline_patch_is_forwarded_as_one_argument_without_stdin() {
        let directory = unique_temp_directory().unwrap();
        let script_path = directory.join("capture.ps1");
        let capture_path = directory.join("capture.json");
        fs::write(&script_path, capture_script()).unwrap();
        let runner = PatchRunner::new(ApplyPatchCommand::new(
            PathBuf::from("powershell.exe"),
            vec![
                String::from("-NoProfile"),
                String::from("-ExecutionPolicy"),
                String::from("Bypass"),
                String::from("-File"),
                script_path.display().to_string(),
                capture_path.display().to_string(),
            ],
        ));
        let patch = [
            "*** Begin Patch",
            "*** Update File: src/main.rs",
            "@@",
            "-println!(\"old\");",
            "+println!(\"new\");",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let output = runner
            .apply(PatchExecution {
                patch: &patch,
                cwd: &directory,
            })
            .await
            .unwrap();
        assert!(output.succeeded(), "{}", output.render());
        let capture = fs::read_to_string(&capture_path).unwrap();
        let invocation: CapturedInvocation = serde_json::from_str(&capture).unwrap();
        assert_eq!(invocation.patch, patch);
        assert_eq!(invocation.stdin, "");
        assert_eq!(invocation.extra_args, 0);
        fs::remove_dir_all(directory).unwrap();
    }
    fn unique_temp_directory() -> anyhow::Result<PathBuf> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory =
            std::env::temp_dir().join(format!("apply-patch-mcp-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }
    fn capture_script() -> &'static str {
        "
param(
    [string]$CapturePath,
    [string]$Patch
)
$stdinContent = [Console]::In.ReadToEnd()
[pscustomobject]@{
    patch = $Patch
    stdin = $stdinContent
    extra_args = $args.Count
} | ConvertTo-Json -Compress | ForEach-Object {
    $encoding = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($CapturePath, $_, $encoding)
}
"
    }
}
