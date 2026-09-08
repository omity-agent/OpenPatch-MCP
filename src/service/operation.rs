mod apply;
mod file_state;
pub(crate) mod model;
mod result;
mod store;
pub(crate) use result::{OperationOutput, PatchToolOutput};
use store::HistoryStore;
#[derive(Debug, Clone)]
pub(crate) struct OperationService {
    history: alloc::sync::Arc<HistoryStore>,
}
impl OperationService {
    pub(crate) fn open_default() -> anyhow::Result<Self> {
        Self::from_history(HistoryStore::open_default()?)
    }
    #[cfg(test)]
    pub(crate) fn open(database_path: &std::path::Path) -> anyhow::Result<Self> {
        Self::from_history(HistoryStore::open(database_path)?)
    }
    fn from_history(history: alloc::sync::Arc<HistoryStore>) -> anyhow::Result<Self> {
        let service = Self { history };
        drop(service.begin()?);
        Ok(service)
    }
    fn begin(&self) -> anyhow::Result<std::fs::File> {
        let guard = self.history.lock()?;
        apply::durable::resume(&self.history)?;
        Ok(guard)
    }
    pub(crate) fn apply(&self, patch: &str) -> OperationOutput {
        apply::execute(self, patch)
    }
    pub(crate) fn replace(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> OperationOutput {
        apply::execute_replacement(self, path, old_string, new_string)
    }
    pub(crate) fn undo(&self, ids: &[String]) -> OperationOutput {
        apply::undo::execute(self, ids)
    }
}
#[cfg(test)]
#[path = "../../test/service/operation.rs"]
mod tests;
