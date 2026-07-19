use crate::operation::OperationService;
#[derive(Debug, Clone)]
pub(crate) struct PatchRunner {
    service: OperationService,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchOutput {
    succeeded: bool,
    content: String,
}
#[derive(Debug, Copy, Clone)]
pub(crate) struct PatchExecution<'request> {
    pub(crate) patch: &'request str,
}
#[derive(Debug, Copy, Clone)]
pub(crate) struct UndoExecution<'request> {
    pub(crate) uuids: &'request [String],
}
impl PatchRunner {
    pub(crate) fn open_default() -> anyhow::Result<Self> {
        Ok(Self {
            service: OperationService::open_default()?,
        })
    }
    #[cfg(test)]
    pub(crate) fn open(database_path: &std::path::Path) -> anyhow::Result<Self> {
        Ok(Self {
            service: OperationService::open(database_path)?,
        })
    }
    pub(crate) fn apply(&self, request: PatchExecution<'_>) -> PatchOutput {
        let output = self.service.apply(request.patch);
        PatchOutput::from_operation(&output)
    }
    pub(crate) fn undo(&self, request: UndoExecution<'_>) -> PatchOutput {
        let output = self.service.undo(request.uuids);
        PatchOutput::from_operation(&output)
    }
}
impl PatchOutput {
    fn from_operation(output: &crate::operation::OperationOutput) -> Self {
        Self {
            succeeded: output.succeeded(),
            content: output.render(),
        }
    }
    pub(crate) const fn succeeded(&self) -> bool {
        self.succeeded
    }
    pub(crate) fn render(&self) -> &str {
        &self.content
    }
}
