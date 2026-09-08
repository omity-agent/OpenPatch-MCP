use super::{HistoryStore, native_path, payload::Digest};
use crate::operation::model::{
    FileState, Mutation, OperationId, PathChange, PathRole, StoredOperation,
};
use heed::RoTxn;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Record {
    pub(super) sequence: u64,
    pub(super) undo_of: Option<i64>,
    pub(super) undone_by: Option<i64>,
    pub(super) changes: Vec<Artifact>,
}
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Artifact {
    pub(super) role: PathRole,
    #[serde(with = "serde_bytes")]
    pub(super) path: Vec<u8>,
    pub(super) before: Option<Digest>,
    pub(super) after: Option<Digest>,
}
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Journal {
    pub(super) records: Vec<(i64, Record)>,
    pub(super) files: Vec<Artifact>,
}
pub(crate) struct Transition {
    pub(crate) path: PathBuf,
    pub(crate) before: FileState,
    pub(crate) after: FileState,
}
impl HistoryStore {
    pub(crate) fn load(&self, id: OperationId) -> anyhow::Result<StoredOperation> {
        let transaction = self.env.read_txn()?;
        let record = self.available(&transaction, id)?;
        let changes = record
            .changes
            .into_iter()
            .map(|artifact| {
                Ok(PathChange {
                    role: artifact.role,
                    path: native_path::decode(&artifact.path)?,
                    before: self.read_content(&transaction, artifact.before)?,
                    after: self.read_content(&transaction, artifact.after)?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(StoredOperation { changes })
    }
    pub(super) fn available(
        &self,
        transaction: &RoTxn<'_>,
        id: OperationId,
    ) -> anyhow::Result<Record> {
        let record = self
            .operations
            .get(transaction, &id.value())?
            .ok_or_else(|| anyhow::anyhow!("unknown operation ID: {id}"))?;
        anyhow::ensure!(
            (1..=2).contains(&record.changes.len()),
            "invalid file state count in operation history"
        );
        if let Some(value) = record.undone_by {
            let undo_id = OperationId::from_stored(value)?;
            anyhow::bail!("operation ID {id} was already undone by {undo_id}");
        }
        Ok(record)
    }
    pub(crate) fn for_undo(&self, ids: &[OperationId]) -> anyhow::Result<Vec<usize>> {
        let transaction = self.env.read_txn()?;
        let mut ordered = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let sequence = self
                    .operations
                    .get(&transaction, &id.value())?
                    .map(|record| record.sequence);
                Ok((index, sequence))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        ordered.sort_by_key(|&(index, sequence)| (core::cmp::Reverse(sequence), index));
        Ok(ordered.into_iter().map(|(index, _)| index).collect())
    }
    pub(super) fn record(
        &self,
        transaction: &mut heed::RwTxn<'_>,
        sequence: u64,
        mutation: &Mutation,
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<Record> {
        let changes = mutation
            .changes
            .iter()
            .map(|change| {
                Ok(Artifact {
                    role: change.role,
                    path: native_path::encode(&change.path),
                    before: self.retain_content(transaction, &change.before)?,
                    after: self.retain_content(transaction, &change.after)?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Record {
            sequence,
            undo_of: undo_of.map(OperationId::value),
            undone_by: None,
            changes,
        })
    }
}
