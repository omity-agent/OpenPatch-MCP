use super::{
    HistoryStore, Transition,
    catalog::{Artifact, Journal},
    ledger, native_path,
};
use crate::operation::model::{Mutation, OperationId, PathRole};
use anyhow::Context as _;
impl HistoryStore {
    pub(crate) fn prepare(
        &self,
        mutations: &[Mutation],
        files: &[Transition],
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<Vec<OperationId>> {
        anyhow::ensure!(
            !mutations.is_empty(),
            "cannot prepare an empty operation batch"
        );
        let mut transaction = self.env.write_txn()?;
        anyhow::ensure!(
            self.pending.is_empty(&transaction)?,
            "operation recovery is required"
        );
        if let Some(target) = undo_of {
            anyhow::ensure!(mutations.len() == 1, "undo must prepare one operation");
            self.available(&transaction, target)?;
        }
        let mut sequence = self.sequence.get(&transaction, &0)?.unwrap_or(0);
        let mut ids = Vec::with_capacity(mutations.len());
        let mut records = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            let id = ledger::allocate(self, &transaction, &ids, OperationId::random)?;
            sequence = sequence
                .checked_add(1)
                .context("operation sequence overflow")?;
            records.push((
                id.value(),
                self.record(&mut transaction, sequence, mutation, undo_of)?,
            ));
            ids.push(id);
        }
        let artifacts = files
            .iter()
            .map(|file| {
                Ok(Artifact {
                    role: PathRole::Single,
                    path: native_path::encode(&file.path),
                    before: self.retain_content(&mut transaction, &file.before)?,
                    after: self.retain_content(&mut transaction, &file.after)?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        self.pending.put(
            &mut transaction,
            &0,
            &Journal {
                records,
                files: artifacts,
            },
        )?;
        self.sequence.put(&mut transaction, &0, &sequence)?;
        transaction
            .commit()
            .context("failed to persist operation intent")?;
        Ok(ids)
    }
    pub(crate) fn recovery(&self) -> anyhow::Result<Option<Vec<Transition>>> {
        let transaction = self.env.read_txn()?;
        let Some(journal) = self
            .pending
            .get(&transaction, &0)
            .context("failed to read recovery intent")?
        else {
            return Ok(None);
        };
        let transitions = journal
            .files
            .into_iter()
            .map(|artifact| {
                Ok(Transition {
                    path: native_path::decode(&artifact.path)?,
                    before: self.read_content(&transaction, artifact.before)?,
                    after: self.read_content(&transaction, artifact.after)?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Some(transitions))
    }
    pub(crate) fn finish(&self) -> anyhow::Result<Vec<OperationId>> {
        let mut transaction = self.env.write_txn()?;
        let ids = self.finalize(&mut transaction)?;
        transaction
            .commit()
            .context("failed to finalize operation history; recovery is required")?;
        Ok(ids)
    }
    pub(super) fn finalize(
        &self,
        transaction: &mut heed::RwTxn<'_>,
    ) -> anyhow::Result<Vec<OperationId>> {
        let journal = self
            .pending
            .get(transaction, &0)?
            .context("operation intent is missing")?;
        let mut ids = Vec::with_capacity(journal.records.len());
        for (value, record) in journal.records {
            let id = OperationId::from_stored(value)?;
            if let Some(target) = record.undo_of {
                let original = OperationId::from_stored(target)?;
                let mut consumed = self.available(transaction, original)?;
                consumed.undone_by = Some(value);
                self.operations.put(transaction, &target, &consumed)?;
            }
            self.chronology.put(transaction, &record.sequence, &value)?;
            self.operations.put(transaction, &value, &record)?;
            ids.push(id);
        }
        for artifact in journal.files {
            self.release_content(transaction, artifact.before)?;
            self.release_content(transaction, artifact.after)?;
        }
        self.pending.delete(transaction, &0)?;
        self.prune(transaction)?;
        Ok(ids)
    }
}
