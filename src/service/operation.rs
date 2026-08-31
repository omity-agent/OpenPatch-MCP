mod apply;
mod file_state;
pub(crate) mod model;
mod result;
mod store;
pub(crate) use result::{OperationOutput, PatchToolOutput};
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
    pub(crate) fn replace(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> OperationOutput {
        apply::execute_replacement(self, path, old_string, new_string)
    }
    pub(crate) fn undo(&self, uuids: &[String]) -> OperationOutput {
        apply::undo::execute(self, uuids)
    }
}
#[cfg(test)]
#[path = "../../test/service/operation.rs"]
mod tests;
