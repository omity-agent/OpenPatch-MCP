use super::model::{Mutation, OperationId, PathChange, PathRole, StoredOperation};
use alloc::sync::Arc;
use anyhow::Context as _;
use diesel::{
    OptionalExtension as _, RunQueryDsl as _,
    sql_types::{BigInt, Binary, Nullable},
    sqlite::SqliteConnection,
};
use smallvec::SmallVec;
use std::{path::PathBuf, sync::Mutex};
mod codec;
mod connection;
mod content;
mod ordering;
mod path;
pub(crate) mod rows;
mod schema;
#[cfg(test)]
#[path = "../../../test/service/store.rs"]
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
    pub(super) fn insert(
        &self,
        connection: &mut SqliteConnection,
        mutation: &Mutation,
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let uuid = OperationId::now_v7();
        let undo_bytes = undo_of.map(|value| value.as_bytes().to_vec());
        let display_path = path::encode(&mutation.display_path);
        diesel::sql_query(
            "INSERT INTO operations(uuid, kind, display_path, undo_of) VALUES (?, ?, ?, ?)",
        )
        .bind::<Binary, _>(uuid.as_bytes().as_slice())
        .bind::<BigInt, _>(mutation.kind.code())
        .bind::<Binary, _>(display_path)
        .bind::<Nullable<Binary>, _>(undo_bytes)
        .execute(connection)
        .with_context(|| format!("failed to write history database: {}", self.path.display()))?;
        let mut writer = content::Writer::new(connection);
        for (ordinal, change) in mutation.changes.iter().enumerate() {
            let ordinal_value =
                i64::try_from(ordinal).context("too many paths in one operation")?;
            let before = writer.store(&change.before)?;
            let after = writer.store(&change.after)?;
            let role = change.role.code();
            let path = path::encode(&change.path);
            diesel::sql_query("INSERT INTO operation_files VALUES (?, ?, ?, ?, ?, ?)")
                .bind::<Binary, _>(uuid.as_bytes().as_slice())
                .bind::<BigInt, _>(ordinal_value)
                .bind::<BigInt, _>(role)
                .bind::<Binary, _>(path)
                .bind::<Nullable<Binary>, _>(before.map(Vec::from))
                .bind::<Nullable<Binary>, _>(after.map(Vec::from))
                .execute(writer.connection())?;
        }
        Ok(uuid)
    }
    pub(super) fn load(
        connection: &mut SqliteConnection,
        id: OperationId,
    ) -> anyhow::Result<StoredOperation> {
        let rows = diesel::sql_query(
            "SELECT role, path, before_digest, after_digest
             FROM operation_files WHERE operation_uuid = ? ORDER BY ordinal",
        )
        .bind::<Binary, _>(id.as_bytes().as_slice())
        .load::<rows::StoredFile>(connection)?;
        let mut changes = SmallVec::new();
        let mut reader = content::Reader::new(connection);
        for row in rows {
            changes.push(PathChange {
                role: PathRole::from_code(row.role)?,
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
        let record = diesel::sql_query("SELECT undone_by FROM operations WHERE uuid = ?")
            .bind::<Binary, _>(id.as_bytes().as_slice())
            .get_result::<rows::UndoneBy>(connection)
            .optional()
            .with_context(|| format!("failed to read history database: {}", self.path.display()))?;
        let Some(availability) = record else {
            anyhow::bail!("unknown operation UUID: {id}");
        };
        if let Some(bytes) = availability.undone_by {
            let undo_uuid = OperationId::from_slice(&bytes)?;
            anyhow::bail!("operation UUID {id} was already undone by {undo_uuid}");
        }
        Ok(())
    }
    pub(super) fn consume_and_insert(
        &self,
        connection: &mut SqliteConnection,
        target: OperationId,
        mutation: &Mutation,
    ) -> anyhow::Result<OperationId> {
        let uuid = self.insert(connection, mutation, Some(target))?;
        let updated = diesel::sql_query(
            "UPDATE operations SET undone_by = ? WHERE uuid = ? AND undone_by IS NULL",
        )
        .bind::<Binary, _>(uuid.as_bytes().as_slice())
        .bind::<Binary, _>(target.as_bytes().as_slice())
        .execute(connection)?;
        if updated != 1 {
            anyhow::bail!("operation UUID {target} was consumed concurrently");
        }
        Ok(uuid)
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
