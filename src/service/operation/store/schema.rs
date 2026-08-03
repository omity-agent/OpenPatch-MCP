use anyhow::Context as _;
use rusqlite::{Connection, TransactionBehavior};
const VERSION: i64 = 1;
const CREATE_SCHEMA : & str = "
CREATE TABLE history_blobs (
    digest BLOB PRIMARY KEY CHECK(typeof(digest) = 'blob' AND length(digest) = 32),
    raw_len INTEGER NOT NULL CHECK(raw_len >= 0),
    codec INTEGER NOT NULL CHECK(codec BETWEEN 0 AND 1),
    payload BLOB NOT NULL CHECK(typeof(payload) = 'blob'),
    CHECK(codec != 0 OR length(payload) = raw_len)
) WITHOUT ROWID;
CREATE TABLE operations (
    sequence INTEGER PRIMARY KEY,
    uuid BLOB NOT NULL UNIQUE CHECK(typeof(uuid) = 'blob' AND length(uuid) = 16),
    kind INTEGER NOT NULL CHECK(kind BETWEEN 0 AND 2),
    display_path BLOB NOT NULL CHECK(typeof(display_path) = 'blob'),
    undo_of BLOB CHECK(undo_of IS NULL OR (typeof(undo_of) = 'blob' AND length(undo_of) = 16)),
    undone_by BLOB CHECK(undone_by IS NULL OR (typeof(undone_by) = 'blob' AND length(undone_by) = 16))
);
CREATE UNIQUE INDEX operations_undo_of ON operations(undo_of) WHERE undo_of IS NOT NULL;
CREATE TABLE operation_files (
    operation_uuid BLOB NOT NULL REFERENCES operations(uuid) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    role INTEGER NOT NULL CHECK(role BETWEEN 0 AND 2),
    path BLOB NOT NULL CHECK(typeof(path) = 'blob'),
    before_digest BLOB REFERENCES history_blobs(digest),
    after_digest BLOB REFERENCES history_blobs(digest),
    PRIMARY KEY(operation_uuid, ordinal),
    CHECK(before_digest IS NULL OR (typeof(before_digest) = 'blob' AND length(before_digest) = 32)),
    CHECK(after_digest IS NULL OR (typeof(after_digest) = 'blob' AND length(after_digest) = 32))
);
CREATE INDEX operation_files_before_digest ON operation_files(before_digest)
    WHERE before_digest IS NOT NULL;
CREATE INDEX operation_files_after_digest ON operation_files(after_digest)
    WHERE after_digest IS NOT NULL;
" ;
pub(super) fn initialize(connection: &mut Connection) -> anyhow::Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match version {
        VERSION => validate(connection),
        0 => create(connection),
        other => anyhow::bail!("unsupported operation history schema version: {other}"),
    }
}
fn create(connection: &mut Connection) -> anyhow::Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == VERSION {
        validate(&transaction)?;
        transaction.commit()?;
        return Ok(());
    }
    anyhow::ensure!(
        version == 0,
        "operation history schema changed while opening"
    );
    let table_count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    anyhow::ensure!(
        table_count == 0,
        "operation history uses an incompatible unversioned schema"
    );
    transaction.execute_batch(CREATE_SCHEMA)?;
    transaction.pragma_update(None, "user_version", VERSION)?;
    transaction.commit()?;
    Ok(())
}
fn validate(connection: &Connection) -> anyhow::Result<()> {
    connection
        .prepare("SELECT digest, raw_len, codec, payload FROM history_blobs LIMIT 0")
        .context("invalid operation history content schema")?;
    connection
        .prepare("SELECT uuid, kind, display_path, undo_of, undone_by FROM operations LIMIT 0")
        .context("invalid operation history operation schema")?;
    connection
        .prepare(
            "SELECT operation_uuid, ordinal, role, path, before_digest, after_digest
             FROM operation_files LIMIT 0",
        )
        .context("invalid operation history file schema")?;
    Ok(())
}
