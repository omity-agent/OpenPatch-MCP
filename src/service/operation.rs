mod apply;
pub(crate) mod files;
mod merge;
pub(crate) mod model;
mod output;
mod store;
mod undo;
pub(crate) use output::{OperationOutput, PatchToolOutput};
use store::HistoryStore;
#[derive(Debug, Clone)]
pub(crate) struct OperationService {
    history: HistoryStore,
}
impl OperationService {
    pub(crate) fn open_default() -> anyhow::Result<Self> {
        Ok(Self {
            history: HistoryStore::open_default()?,
        })
    }
    #[cfg(test)]
    pub(crate) fn open(database_path: &std::path::Path) -> anyhow::Result<Self> {
        Ok(Self {
            history: HistoryStore::open(database_path)?,
        })
    }
    pub(crate) fn apply(&self, patch: &str) -> OperationOutput {
        apply::execute(self, patch)
    }
    pub(crate) fn undo(&self, uuids: &[String]) -> OperationOutput {
        undo::execute(self, uuids)
    }
}
#[cfg(test)]
mod tests;
