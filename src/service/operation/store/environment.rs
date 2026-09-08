use super::HistoryStore;
use alloc::sync::{Arc, Weak};
use anyhow::Context as _;
use directories::ProjectDirs;
use heed::EnvOpenOptions;
use rustc_hash::FxHashMap;
use std::{
    fs::{self, File},
    path::Path,
    sync::{LazyLock, Mutex},
};
static OPEN_STORES: LazyLock<Mutex<FxHashMap<std::path::PathBuf, Weak<HistoryStore>>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));
impl HistoryStore {
    pub(crate) fn open_default() -> anyhow::Result<Arc<Self>> {
        let directories = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
            .context("failed to locate the user application data directory")?;
        Self::open(&directories.data_local_dir().join("history.lmdb"))
    }
    pub(crate) fn open(path: &Path) -> anyhow::Result<Arc<Self>> {
        let capacity = match std::env::var("OPENPATCH_HISTORY_MAP_MIB") {
            Ok(value) => value
                .parse::<usize>()
                .context("OPENPATCH_HISTORY_MAP_MIB must be a positive integer")?,
            Err(std::env::VarError::NotPresent) => 1024,
            Err(error) => return Err(error.into()),
        };
        anyhow::ensure!(
            capacity >= 64,
            "OPENPATCH_HISTORY_MAP_MIB must be at least 64"
        );
        let map_size = capacity
            .checked_mul(1024 * 1024)
            .context("history map size overflow")?;
        Self::open_capacity(path, map_size)
    }
    pub(super) fn open_capacity(path: &Path, map_size: usize) -> anyhow::Result<Arc<Self>> {
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create history directory: {}", path.display()))?;
        let canonical = path.canonicalize()?;
        let mut registry = OPEN_STORES
            .lock()
            .map_err(|error| anyhow::anyhow!("history registry lock is poisoned: {error}"))?;
        if let Some(weak) = registry.get(&canonical)
            && let Some(existing) = weak.upgrade()
        {
            anyhow::ensure!(
                existing.env.info().map_size == map_size,
                "close all history handles before changing the map size"
            );
            return Ok(existing);
        }
        registry.retain(|_, store| store.strong_count() != 0);
        if let Some(closing) = heed::env_closing_event(&canonical) {
            closing.wait();
        }
        let guard = lock_directory(&canonical)?;
        anyhow::ensure!(
            canonical.to_str().is_some(),
            "history path must be valid Unicode"
        );
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(map_size)
                .max_dbs(6)
                .open(&canonical)?
        };
        let stale_readers = env.clear_stale_readers()?;
        if stale_readers != 0 {
            eprintln!("warning: cleared {stale_readers} stale history readers");
        }
        let mut transaction = env.write_txn()?;
        let store = Self {
            path: canonical,
            operations: env.create_database(&mut transaction, Some("operations"))?,
            chronology: env.create_database(&mut transaction, Some("chronology"))?,
            contents: env.create_database(&mut transaction, Some("contents"))?,
            references: env.create_database(&mut transaction, Some("references"))?,
            pending: env.create_database(&mut transaction, Some("pending"))?,
            sequence: env.create_database(&mut transaction, Some("sequence"))?,
            env: env.clone(),
        };
        transaction.commit()?;
        let shared = Arc::new(store);
        registry.insert(shared.path.clone(), Arc::downgrade(&shared));
        drop(guard);
        drop(registry);
        Ok(shared)
    }
    pub(crate) fn lock(&self) -> anyhow::Result<File> {
        lock_directory(&self.path)
    }
}
fn lock_directory(path: &Path) -> anyhow::Result<File> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.join("operations.lock"))?;
    file.lock()
        .with_context(|| format!("failed to lock operation history: {}", path.display()))?;
    Ok(file)
}
