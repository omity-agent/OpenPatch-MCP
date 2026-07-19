use super::model::{FileState, Mutation, OperationId, PathChange, PathRole, StoredOperation};
use anyhow::Context as _;
use core::time::Duration;
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension as _, Transaction, params};
use std::path::{Path, PathBuf};
type SqlTransaction<'connection> = Transaction<'connection>;
const RETAINED_OPERATIONS: i64 = 100;
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS operations (sequence INTEGER PRIMARY KEY, uuid BLOB NOT NULL UNIQUE CHECK(length(uuid)=16), kind INTEGER NOT NULL CHECK(kind BETWEEN 0 AND 2), display_path BLOB NOT NULL, undo_of BLOB CHECK(undo_of IS NULL OR length(undo_of)=16), undone_by BLOB CHECK(undone_by IS NULL OR length(undone_by)=16)); CREATE UNIQUE INDEX IF NOT EXISTS operations_undo_of ON operations(undo_of) WHERE undo_of IS NOT NULL; CREATE TABLE IF NOT EXISTS operation_files (operation_uuid BLOB NOT NULL REFERENCES operations(uuid) ON DELETE CASCADE, ordinal INTEGER NOT NULL, role INTEGER NOT NULL CHECK(role BETWEEN 0 AND 2), path BLOB NOT NULL, before_present INTEGER NOT NULL, before_contents TEXT, after_present INTEGER NOT NULL, after_contents TEXT, PRIMARY KEY(operation_uuid, ordinal), CHECK((before_present=0 AND before_contents IS NULL) OR (before_present=1 AND before_contents IS NOT NULL)), CHECK((after_present=0 AND after_contents IS NULL) OR (after_present=1 AND after_contents IS NOT NULL)));";
#[derive(Debug, Clone)]
pub(super) struct HistoryStore {
    path: PathBuf,
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
        let store = Self { path: path.into() };
        let connection = store.connection()?;
        connection.execute_batch(SCHEMA)?;
        Ok(store)
    }
    pub(super) fn connection(&self) -> anyhow::Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(30))?;
        let mode: String =
            connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            anyhow::bail!("failed to enable SQLite WAL mode");
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(connection)
    }
    pub(super) fn insert(
        &self,
        tx: &SqlTransaction<'_>,
        mutation: &Mutation,
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let uuid = OperationId::now_v7();
        let undo_bytes = undo_of.map(|value| value.as_bytes().to_vec());
        let display_path = encode_path(&mutation.display_path);
        let operation_values = (
            uuid.as_bytes(),
            mutation.kind.code(),
            display_path,
            undo_bytes,
        );
        tx.execute(
            "INSERT INTO operations(uuid, kind, display_path, undo_of) VALUES (?1, ?2, ?3, ?4)",
            operation_values,
        )
        .with_context(|| format!("failed to write history database: {}", self.path.display()))?;
        for (ordinal, change) in mutation.changes.iter().enumerate() {
            let ordinal_value =
                i64::try_from(ordinal).context("too many paths in one operation")?;
            let before = change.before.database_parts();
            let after = change.after.database_parts();
            let role = change.role.code();
            let path = encode_path(&change.path);
            let file_values = (
                uuid.as_bytes(),
                ordinal_value,
                role,
                path,
                before.0,
                before.1,
                after.0,
                after.1,
            );
            tx.execute(
                "INSERT INTO operation_files VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                file_values,
            )?;
        }
        Ok(uuid)
    }
    pub(super) fn load(
        &self,
        tx: &SqlTransaction<'_>,
        id: OperationId,
    ) -> anyhow::Result<StoredOperation> {
        self.ensure_available(tx, id)?;
        let mut statement = tx.prepare(
            "SELECT role, path, before_present, before_contents, after_present, after_contents
             FROM operation_files WHERE operation_uuid = ?1 ORDER BY ordinal",
        )?;
        let mut rows = statement.query(params![id.as_bytes()])?;
        let mut changes = Vec::new();
        while let Some(row) = rows.next()? {
            changes.push(PathChange {
                role: PathRole::from_code(row.get(0)?)?,
                path: decode_path(&row.get::<_, Vec<u8>>(1)?)?,
                before: FileState::from_database(row.get(2)?, row.get(3)?)?,
                after: FileState::from_database(row.get(4)?, row.get(5)?)?,
            });
        }
        if changes.is_empty() {
            anyhow::bail!("operation history contains no file states");
        }
        Ok(StoredOperation { changes })
    }
    pub(super) fn ensure_available(
        &self,
        tx: &SqlTransaction<'_>,
        id: OperationId,
    ) -> anyhow::Result<()> {
        let record_undone_by = tx
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
        tx: &SqlTransaction<'_>,
        target: OperationId,
        mutation: &Mutation,
    ) -> anyhow::Result<OperationId> {
        self.ensure_available(tx, target)?;
        let uuid = self.insert(tx, mutation, Some(target))?;
        let updated = tx.execute(
            "UPDATE operations SET undone_by = ?1 WHERE uuid = ?2 AND undone_by IS NULL",
            params![uuid.as_bytes(), target.as_bytes()],
        )?;
        if updated != 1 {
            anyhow::bail!("operation UUID {target} was consumed concurrently");
        }
        Ok(uuid)
    }
    pub(super) fn prune(&self, tx: &SqlTransaction<'_>) -> anyhow::Result<()> {
        tx.execute(
            "DELETE FROM operations WHERE sequence IN (
                SELECT sequence FROM operations ORDER BY sequence DESC LIMIT -1 OFFSET ?1
            )",
            [RETAINED_OPERATIONS],
        )
        .with_context(|| format!("failed to prune history database: {}", self.path.display()))?;
        Ok(())
    }
}
#[cfg(unix)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt as _;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(unix)]
fn decode_path(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;
    Ok(std::ffi::OsString::from_vec(bytes.to_vec()).into())
}
#[cfg(windows)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt as _;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_be_bytes)
        .collect()
}
#[cfg(windows)]
fn decode_path(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt as _;
    let (pairs, remainder) = bytes.as_chunks::<2>();
    anyhow::ensure!(remainder.is_empty(), "invalid path in operation history");
    let wide = pairs
        .iter()
        .map(|pair| u16::from_be_bytes(*pair))
        .collect::<Vec<_>>();
    Ok(std::ffi::OsString::from_wide(&wide).into())
}
