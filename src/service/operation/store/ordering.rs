use super::HistoryStore;
use crate::operation::model::OperationId;
use core::cmp::Ordering;
use diesel::{OptionalExtension as _, RunQueryDsl as _, sql_types::Binary};
pub(crate) fn for_undo(store: &HistoryStore, ids: &[OperationId]) -> anyhow::Result<Vec<usize>> {
    let mut ordered = {
        let mut connection = store.lock_connection()?;
        let queried = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let sequence =
                    diesel::sql_query("SELECT sequence AS value FROM operations WHERE uuid = ?")
                        .bind::<Binary, _>(id.as_bytes().as_slice())
                        .get_result::<super::rows::IntegerValue>(&mut *connection)
                        .optional()?
                        .map(|row| row.value);
                Ok((index, sequence))
            })
            .collect::<diesel::QueryResult<Vec<_>>>()?;
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
