use super::model::{Mutation, OperationId, PathChange, PathRole, StoredOperation};
use alloc::sync::Arc;
use anyhow::Context as _;
use core::time::Duration;
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension as _, params};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
mod content;
mod ordering;
mod path;
mod schema;
#[cfg(test)]
mod tests;
pub(super) use ordering::for_undo;
const RETAINED_OPERATIONS: i64 = 1000;
#[derive(Debug, Clone)]
pub(super) struct HistoryStore {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}
impl HistoryStore {
    pub(super) fn open_default() -> anyhow::Result<Self> {
        let directories = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
            .context("failed to locate the user application data directory")?;
        Self::open(&directories.data_local_dir().join("history.sqlite3"))
    }
    pub(super) fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create history directory: {}", parent.display())
            })?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(30))?;
        let mode: String =
            connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            anyhow::bail!("failed to enable SQLite WAL mode");
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        schema::initialize(&mut connection)?;
        Ok(Self {
            path: path.into(),
            connection: Arc::new(Mutex::new(connection)),
        })
    }
    pub(super) fn lock_connection(&self) -> anyhow::Result<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|error| {
            anyhow::anyhow!(
                "operation history connection lock is poisoned for {}: {error}",
                self.path.display(),
            )
        })
    }
    pub(super) fn insert(
        &self,
        connection: &Connection,
        mutation: &Mutation,
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let uuid = OperationId::now_v7();
        let undo_bytes = undo_of.map(|value| value.as_bytes().to_vec());
        let display_path = path::encode(&mutation.display_path);
        let operation_values = (
            uuid.as_bytes(),
            mutation.kind.code(),
            display_path,
            undo_bytes,
        );
        connection
            .execute(
                "INSERT INTO operations(uuid, kind, display_path, undo_of) VALUES (?1, ?2, ?3, ?4)",
                operation_values,
            )
            .with_context(|| {
                format!("failed to write history database: {}", self.path.display())
            })?;
        let mut writer = content::Writer::new(connection);
        for (ordinal, change) in mutation.changes.iter().enumerate() {
            let ordinal_value =
                i64::try_from(ordinal).context("too many paths in one operation")?;
            let before = writer.store(&change.before)?;
            let after = writer.store(&change.after)?;
            let role = change.role.code();
            let path = path::encode(&change.path);
            let file_values = (
                uuid.as_bytes(),
                ordinal_value,
                role,
                path,
                before.as_ref().map(<[u8; blake3::OUT_LEN]>::as_slice),
                after.as_ref().map(<[u8; blake3::OUT_LEN]>::as_slice),
            );
            connection.execute(
                "INSERT INTO operation_files VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                file_values,
            )?;
        }
        Ok(uuid)
    }
    pub(super) fn load(
        connection: &Connection,
        id: OperationId,
    ) -> anyhow::Result<StoredOperation> {
        let mut statement = connection.prepare(
            "SELECT role, path, before_digest, after_digest
             FROM operation_files WHERE operation_uuid = ?1 ORDER BY ordinal",
        )?;
        let mut rows = statement.query(params![id.as_bytes()])?;
        let mut changes = Vec::new();
        let mut reader = content::Reader::new(connection);
        while let Some(row) = rows.next()? {
            changes.push(PathChange {
                role: PathRole::from_code(row.get(0)?)?,
                path: path::decode(&row.get::<_, Vec<u8>>(1)?)?,
                before: reader.load(row.get(2)?)?,
                after: reader.load(row.get(3)?)?,
            });
        }
        if changes.is_empty() {
            anyhow::bail!("operation history contains no file states");
        }
        Ok(StoredOperation { changes })
    }
    pub(super) fn ensure_available(
        &self,
        connection: &Connection,
        id: OperationId,
    ) -> anyhow::Result<()> {
        let record_undone_by = connection
            .query_row(
                "SELECT undone_by FROM operations WHERE uuid = ?1",
                params![id.as_bytes()],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()
            .with_context(|| format!("failed to read history database: {}", self.path.display()))?;
        let Some(undone_by) = record_undone_by else {
            anyhow::bail!("unknown operation UUID: {id}");
        };
        if let Some(bytes) = undone_by {
            let undo_uuid = OperationId::from_slice(&bytes)?;
            anyhow::bail!("operation UUID {id} was already undone by {undo_uuid}");
        }
        Ok(())
    }
    pub(super) fn consume_and_insert(
        &self,
        connection: &Connection,
        target: OperationId,
        mutation: &Mutation,
    ) -> anyhow::Result<OperationId> {
        let uuid = self.insert(connection, mutation, Some(target))?;
        let updated = connection.execute(
            "UPDATE operations SET undone_by = ?1 WHERE uuid = ?2 AND undone_by IS NULL",
            params![uuid.as_bytes(), target.as_bytes()],
        )?;
        if updated != 1 {
            anyhow::bail!("operation UUID {target} was consumed concurrently");
        }
        Ok(uuid)
    }
    pub(super) fn prune(&self, connection: &Connection) -> anyhow::Result<()> {
        let deleted = connection
            .execute(
                "DELETE FROM operations WHERE sequence IN (
                SELECT sequence FROM operations ORDER BY sequence DESC LIMIT -1 OFFSET ?1
            )",
                [RETAINED_OPERATIONS],
            )
            .with_context(|| {
                format!("failed to prune history database: {}", self.path.display())
            })?;
        if deleted != 0 {
            content::delete_unreferenced(connection)?;
        }
        Ok(())
    }
}
