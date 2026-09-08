use super::{HistoryStore, RETAINED_OPERATIONS};
use crate::operation::model::OperationId;
use anyhow::Context as _;
use heed::RoTxn;
impl HistoryStore {
    pub(super) fn prune(&self, transaction: &mut heed::RwTxn<'_>) -> anyhow::Result<()> {
        while self.chronology.len(transaction)? > RETAINED_OPERATIONS {
            let (sequence, id) = self
                .chronology
                .first(transaction)?
                .context("operation chronology is unexpectedly empty")?;
            let record = self
                .operations
                .get(transaction, &id)?
                .context("operation chronology references missing history")?;
            for change in record.changes {
                self.release_content(transaction, change.before)?;
                self.release_content(transaction, change.after)?;
            }
            self.operations.delete(transaction, &id)?;
            self.chronology.delete(transaction, &sequence)?;
        }
        Ok(())
    }
}
pub(super) fn allocate(
    store: &HistoryStore,
    transaction: &RoTxn<'_>,
    reserved: &[OperationId],
    mut generate: impl FnMut() -> OperationId,
) -> anyhow::Result<OperationId> {
    const MAX_ATTEMPTS: usize = 16;
    for _ in 0..MAX_ATTEMPTS {
        let id = generate();
        if !reserved.contains(&id) && store.operations.get(transaction, &id.value())?.is_none() {
            return Ok(id);
        }
        eprintln!("warning: operation ID collision: {id}");
    }
    anyhow::bail!("failed to allocate a unique operation ID after {MAX_ATTEMPTS} attempts")
}
