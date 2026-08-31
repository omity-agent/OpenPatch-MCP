#![expect(
    clippy::redundant_field_names,
    reason = "Diesel's QueryableByName derive generates explicit field names"
)]
use diesel::{
    QueryableByName,
    sql_types::{BigInt, Binary, Nullable, Text},
};
#[derive(QueryableByName)]
pub(super) struct JournalMode {
    # [diesel (sql_type = Text)]
    pub(super) journal_mode: String,
}
#[derive(QueryableByName)]
pub(super) struct StoredFile {
    # [diesel (sql_type = BigInt)]
    pub(super) role: i64,
    # [diesel (sql_type = Binary)]
    pub(super) path: Vec<u8>,
    # [diesel (sql_type = Nullable < Binary >)]
    pub(super) before_digest: Option<Vec<u8>>,
    # [diesel (sql_type = Nullable < Binary >)]
    pub(super) after_digest: Option<Vec<u8>>,
}
#[derive(QueryableByName)]
pub(super) struct UndoneBy {
    # [diesel (sql_type = Nullable < Binary >)]
    pub(super) undone_by: Option<Vec<u8>>,
}
#[derive(QueryableByName)]
pub(crate) struct IntegerValue {
    # [diesel (sql_type = BigInt)]
    pub(crate) value: i64,
}
#[derive(QueryableByName)]
pub(super) struct StoredContent {
    # [diesel (sql_type = BigInt)]
    pub(super) raw_len: i64,
    # [diesel (sql_type = BigInt)]
    pub(super) codec: i64,
    # [diesel (sql_type = Binary)]
    pub(super) payload: Vec<u8>,
}
