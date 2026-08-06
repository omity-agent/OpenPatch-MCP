use super::HistoryStore;
use crate::operation::model::OperationId;
use core::cmp::Ordering;
use rusqlite::{OptionalExtension as _, params};
pub(crate) fn for_undo(store: &HistoryStore, ids: &[OperationId]) -> anyhow::Result<Vec<usize>> {
    let mut ordered = {
        let connection = store.lock_connection()?;
        let mut statement =
            connection.prepare("SELECT sequence FROM operations WHERE uuid = ?1")?;
        let queried = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let sequence = statement
                    .query_row(params![id.as_bytes()], |row| row.get::<_, i64>(0))
                    .optional()?;
                Ok((index, sequence))
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        drop(connection);
        queried
    };
    ordered.sort_by(
        |&(left_index, left_sequence), &(right_index, right_sequence)| match (
            left_sequence,
            right_sequence,
        ) {
            (Some(stored_left_sequence), Some(stored_right_sequence)) => {
                stored_right_sequence.cmp(&stored_left_sequence)
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left_index.cmp(&right_index),
        },
    );
    Ok(ordered.into_iter().map(|(index, _)| index).collect())
}
