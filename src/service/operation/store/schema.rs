use anyhow::Context as _;
use diesel::{RunQueryDsl as _, connection::SimpleConnection as _, sqlite::SqliteConnection};
const VERSION: i64 = 2;
const CREATE_SCHEMA: &str = "
CREATE TABLE history_blobs (
    digest BLOB PRIMARY KEY CHECK(typeof(digest) = 'blob' AND length(digest) = 32),
    raw_len INTEGER NOT NULL CHECK(raw_len >= 0),
    codec INTEGER NOT NULL CHECK(codec BETWEEN 0 AND 1),
    payload BLOB NOT NULL CHECK(typeof(payload) = 'blob'),
    CHECK(codec != 0 OR length(payload) = raw_len)
) WITHOUT ROWID;
CREATE TABLE operations (
    sequence INTEGER PRIMARY KEY,
    id INTEGER NOT NULL UNIQUE CHECK(typeof(id) = 'integer' AND id BETWEEN 0 AND 2821109907455),
    kind INTEGER NOT NULL CHECK(kind BETWEEN 0 AND 2),
    display_path BLOB NOT NULL CHECK(typeof(display_path) = 'blob'),
    undo_of INTEGER CHECK(undo_of IS NULL OR
        (typeof(undo_of) = 'integer' AND undo_of BETWEEN 0 AND 2821109907455)),
    undone_by INTEGER CHECK(undone_by IS NULL OR
        (typeof(undone_by) = 'integer' AND undone_by BETWEEN 0 AND 2821109907455))
);
CREATE UNIQUE INDEX operations_undo_of ON operations(undo_of) WHERE undo_of IS NOT NULL;
CREATE TABLE operation_files (
    operation_id INTEGER NOT NULL REFERENCES operations(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    role INTEGER NOT NULL CHECK(role BETWEEN 0 AND 2),
    path BLOB NOT NULL CHECK(typeof(path) = 'blob'),
    before_digest BLOB REFERENCES history_blobs(digest),
    after_digest BLOB REFERENCES history_blobs(digest),
    PRIMARY KEY(operation_id, ordinal),
    CHECK(before_digest IS NULL OR (typeof(before_digest) = 'blob' AND length(before_digest) = 32)),
    CHECK(after_digest IS NULL OR (typeof(after_digest) = 'blob' AND length(after_digest) = 32))
);
CREATE INDEX operation_files_before_digest ON operation_files(before_digest)
    WHERE before_digest IS NOT NULL;
CREATE INDEX operation_files_after_digest ON operation_files(after_digest)
    WHERE after_digest IS NOT NULL;
";
pub(super) fn initialize(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    let version = user_version(connection)?;
    match version {
        VERSION => validate(connection),
        0 => create(connection),
        other => anyhow::bail!("unsupported operation history schema version: {other}"),
    }
}
fn user_version(connection: &mut SqliteConnection) -> anyhow::Result<i64> {
    Ok(
        diesel::sql_query("SELECT user_version AS value FROM pragma_user_version")
            .get_result::<super::rows::IntegerValue>(connection)?
            .value,
    )
}
fn create(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    connection.immediate_transaction(|transaction| {
        let version = user_version(transaction)?;
        if version == VERSION {
            return validate(transaction);
        }
        anyhow::ensure!(
            version == 0,
            "operation history schema changed while opening"
        );
        let table_count = diesel::sql_query(
            "SELECT COUNT(*) AS value FROM sqlite_schema
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .get_result::<super::rows::IntegerValue>(transaction)?
        .value;
        anyhow::ensure!(
            table_count == 0,
            "operation history uses an incompatible unversioned schema"
        );
        transaction.batch_execute(CREATE_SCHEMA)?;
        transaction.batch_execute(&format!("PRAGMA user_version = {VERSION}"))?;
        Ok(())
    })
}
fn validate(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    let content_columns: Vec<super::rows::StoredContent> =
        diesel::sql_query("SELECT digest, raw_len, codec, payload FROM history_blobs LIMIT 0")
            .load(connection)
            .context("invalid operation history content schema")?;
    anyhow::ensure!(
        content_columns.is_empty(),
        "operation history schema validation unexpectedly returned content"
    );
    let operation_columns = column_count(connection, "operations")?;
    anyhow::ensure!(
        operation_columns == 6,
        "invalid operation history operation schema"
    );
    let file_columns = column_count(connection, "operation_files")?;
    anyhow::ensure!(file_columns == 6, "invalid operation history file schema");
    Ok(())
}
fn column_count(connection: &mut SqliteConnection, table: &str) -> anyhow::Result<i64> {
    let query = format!("SELECT COUNT(*) AS value FROM pragma_table_info('{table}')");
    Ok(diesel::sql_query(query)
        .get_result::<super::rows::IntegerValue>(connection)?
        .value)
}
