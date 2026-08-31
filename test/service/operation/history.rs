use super::service;
use crate::operation::model::{FileState, Mutation, OperationKind};
use crate::operation::store::rows::IntegerValue;
use diesel::RunQueryDsl as _;
#[test]
fn retention_uses_only_the_latest_thousand_records() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let mut connection = service.history.lock_connection().unwrap();
    connection
        .immediate_transaction(|transaction| {
            for index in 0_u16..1_001 {
                let mutation = Mutation::single(
                    OperationKind::Edit,
                    directory.path().join(format!("{index}.txt")),
                    FileState::Missing,
                    FileState::Missing,
                );
                service.history.insert(transaction, &mutation, None)?;
                service.history.prune(transaction)?;
            }
            let count = diesel::sql_query("SELECT COUNT(*) AS value FROM operations")
                .get_result::<IntegerValue>(transaction)?
                .value;
            assert_eq!(count, 1000);
            anyhow::Ok(())
        })
        .unwrap();
    drop(connection);
}
