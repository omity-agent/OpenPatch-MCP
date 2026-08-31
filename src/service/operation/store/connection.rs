use super::{HistoryStore, rows, schema};
use alloc::sync::Arc;
use anyhow::Context as _;
use core::fmt;
use diesel::{
    Connection as _, RunQueryDsl as _,
    connection::{AnsiTransactionManager, SimpleConnection as _, TransactionManager as _},
    sqlite::SqliteConnection,
};
use directories::ProjectDirs;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};
impl HistoryStore {
    pub(crate) fn open_default() -> anyhow::Result<Self> {
        let directories = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
            .context("failed to locate the user application data directory")?;
        Self::open(&directories.data_local_dir().join("history.sqlite3"))
    }
    pub(crate) fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create history directory: {}", parent.display())
            })?;
        }
        let database_url = path
            .to_str()
            .context("operation history path is not valid Unicode")?;
        let mut connection = SqliteConnection::establish(database_url)?;
        connection.batch_execute(
            "PRAGMA busy_timeout = 30000;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;",
        )?;
        let mode = diesel::sql_query("PRAGMA journal_mode = WAL")
            .get_result::<rows::JournalMode>(&mut connection)?
            .journal_mode;
        anyhow::ensure!(
            mode.eq_ignore_ascii_case("wal"),
            "failed to enable SQLite WAL mode"
        );
        schema::initialize(&mut connection)?;
        Ok(Self {
            path: path.into(),
            connection: Arc::new(Mutex::new(connection)),
        })
    }
    pub(crate) fn lock_connection(&self) -> anyhow::Result<MutexGuard<'_, SqliteConnection>> {
        self.connection.lock().map_err(|error| {
            anyhow::anyhow!(
                "operation history connection lock is poisoned for {}: {error}",
                self.path.display(),
            )
        })
    }
}
impl fmt::Debug for HistoryStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HistoryStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}
pub(crate) fn roll_back_transaction(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    AnsiTransactionManager::rollback_transaction(connection)?;
    Ok(())
}
