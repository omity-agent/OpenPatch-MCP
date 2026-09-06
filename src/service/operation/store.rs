use super::model::{Mutation, OperationId, PathChange, PathRole, StoredOperation};
use alloc::sync::Arc;
use anyhow::Context as _;
use diesel::{
    OptionalExtension as _, RunQueryDsl as _, sql_types::BigInt, sqlite::SqliteConnection,
};
use smallvec::SmallVec;
use std::{path::PathBuf, sync::Mutex};
mod codec;
mod connection;
mod content;
mod ordering;
mod path;
mod recording;
pub(crate) mod rows;
mod schema;
#[cfg(test)]
#[path = "../../../test/persistence.rs"]
mod tests;
pub(crate) use connection::roll_back_transaction;
pub(super) use ordering::for_undo;
const RETAINED_OPERATIONS: i64 = 1000;
#[derive(Clone)]
pub(super) struct HistoryStore {
    path: PathBuf,
    connection: Arc<Mutex<SqliteConnection>>,
}
impl HistoryStore {
    pub(super) fn load(
        connection: &mut SqliteConnection,
        id: OperationId,
    ) -> anyhow::Result<StoredOperation> {
        let rows = diesel::sql_query(
            "SELECT role, path, before_digest, after_digest
             FROM operation_files WHERE operation_id = ? ORDER BY ordinal",
        )
        .bind::<BigInt, _>(id.value())
        .load::<rows::StoredFile>(connection)?;
        let mut changes = SmallVec::new();
        let mut reader = content::Reader::new(connection);
        for row in rows {
            changes.push(PathChange {
                role: PathRole::try_from(row.role).context("invalid path role in history")?,
                path: path::decode(&row.path)?,
                before: reader.load(row.before_digest)?,
                after: reader.load(row.after_digest)?,
            });
        }
        if changes.is_empty() {
            anyhow::bail!("operation history contains no file states");
        }
        if changes.len() > 2 {
            anyhow::bail!("operation history contains too many file states");
        }
        Ok(StoredOperation { changes })
    }
    pub(super) fn ensure_available(
        &self,
        connection: &mut SqliteConnection,
        id: OperationId,
    ) -> anyhow::Result<()> {
        let record = diesel::sql_query("SELECT undone_by FROM operations WHERE id = ?")
            .bind::<BigInt, _>(id.value())
            .get_result::<rows::UndoneBy>(connection)
            .optional()
            .with_context(|| format!("failed to read history database: {}", self.path.display()))?;
        let Some(availability) = record else {
            anyhow::bail!("unknown operation ID: {id}");
        };
        if let Some(value) = availability.undone_by {
            let undo_id = OperationId::from_stored(value)?;
            anyhow::bail!("operation ID {id} was already undone by {undo_id}");
        }
        Ok(())
    }
    pub(super) fn consume_and_insert(
        &self,
        connection: &mut SqliteConnection,
        target: OperationId,
        mutation: &Mutation,
    ) -> anyhow::Result<OperationId> {
        let id = self.insert(connection, mutation, Some(target))?;
        let updated = diesel::sql_query(
            "UPDATE operations SET undone_by = ? WHERE id = ? AND undone_by IS NULL",
        )
        .bind::<BigInt, _>(id.value())
        .bind::<BigInt, _>(target.value())
        .execute(connection)?;
        if updated != 1 {
            anyhow::bail!("operation ID {target} was consumed concurrently");
        }
        Ok(id)
    }
    pub(super) fn prune(&self, connection: &mut SqliteConnection) -> anyhow::Result<()> {
        let deleted = diesel::sql_query(
            "DELETE FROM operations WHERE sequence IN (
                SELECT sequence FROM operations ORDER BY sequence DESC LIMIT -1 OFFSET ?1
            )",
        )
        .bind::<BigInt, _>(RETAINED_OPERATIONS)
        .execute(connection)
        .with_context(|| format!("failed to prune history database: {}", self.path.display()))?;
        if deleted != 0 {
            content::delete_unreferenced(connection)?;
        }
        Ok(())
    }
}
