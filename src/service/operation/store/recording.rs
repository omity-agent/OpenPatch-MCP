use super::{HistoryStore, content, path};
use crate::operation::model::{Mutation, OperationId};
use anyhow::Context as _;
use diesel::{
    RunQueryDsl as _,
    sql_types::{BigInt, Binary, Nullable},
    sqlite::SqliteConnection,
};
impl HistoryStore {
    pub(crate) fn insert(
        &self,
        connection: &mut SqliteConnection,
        mutation: &Mutation,
        undo_of: Option<OperationId>,
    ) -> anyhow::Result<OperationId> {
        let id =
            reserve(connection, mutation, undo_of, OperationId::random).with_context(|| {
                format!("failed to write history database: {}", self.path.display())
            })?;
        let mut writer = content::Writer::new(connection);
        for (ordinal, change) in mutation.changes.iter().enumerate() {
            let ordinal_value =
                i64::try_from(ordinal).context("too many paths in one operation")?;
            let before = writer.store(&change.before)?;
            let after = writer.store(&change.after)?;
            let role = i64::from(change.role);
            let encoded_path = path::encode(&change.path);
            diesel::sql_query("INSERT INTO operation_files VALUES (?, ?, ?, ?, ?, ?)")
                .bind::<BigInt, _>(id.value())
                .bind::<BigInt, _>(ordinal_value)
                .bind::<BigInt, _>(role)
                .bind::<Binary, _>(encoded_path)
                .bind::<Nullable<Binary>, _>(before.map(Vec::from))
                .bind::<Nullable<Binary>, _>(after.map(Vec::from))
                .execute(writer.connection())?;
        }
        Ok(id)
    }
}
pub(super) fn reserve(
    connection: &mut SqliteConnection,
    mutation: &Mutation,
    undo_of: Option<OperationId>,
    mut generate: impl FnMut() -> OperationId,
) -> anyhow::Result<OperationId> {
    const MAX_ATTEMPTS: usize = 16;
    let display_path = path::encode(&mutation.display_path);
    for _ in 0..MAX_ATTEMPTS {
        let id = generate();
        let inserted = diesel::sql_query(
            "INSERT INTO operations(id, kind, display_path, undo_of) VALUES (?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind::<BigInt, _>(id.value())
        .bind::<BigInt, _>(i64::from(mutation.kind))
        .bind::<Binary, _>(&display_path)
        .bind::<Nullable<BigInt>, _>(undo_of.map(OperationId::value))
        .execute(connection)?;
        match inserted {
            1 => return Ok(id),
            0 => eprintln!("warning: operation ID collision: {id}"),
            other => anyhow::bail!("unexpected operation history insert count: {other}"),
        }
    }
    anyhow::bail!("failed to allocate a unique operation ID after {MAX_ATTEMPTS} attempts")
}
