use crate::patch::{ApplyResult, apply_patch_text};
#[derive(Debug, Clone, Default)]
pub struct PatchRunner;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOutput {
    succeeded: bool,
    content: String,
}
#[derive(Debug, Copy, Clone)]
pub struct PatchExecution<'request> {
    pub patch: &'request str,
}
impl PatchRunner {
    pub fn apply(request: PatchExecution<'_>) -> PatchOutput {
        let ApplyResult { output, succeeded } = apply_patch_text(request.patch);
        PatchOutput {
            succeeded,
            content: output,
        }
    }
}
impl PatchOutput {
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        self.succeeded
    }
    #[must_use]
    pub fn render(&self) -> &str {
        &self.content
    }
}
#[cfg(test)]
mod tests;
