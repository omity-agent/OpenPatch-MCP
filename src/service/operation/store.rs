use catalog::{Journal, Record};
use heed::{
    Database, Env,
    byteorder::BigEndian,
    types::{Bytes, I64, SerdeRmp, U8, U64},
};
use payload::Blob;
use std::path::PathBuf;
mod catalog;
mod environment;
mod intent;
mod ledger;
mod native_path;
mod payload;
pub(super) use catalog::Transition;
#[cfg(test)]
#[path = "../../../test/persistence.rs"]
mod tests;
const RETAINED_OPERATIONS: u64 = 1000;
type IdKey = I64<BigEndian>;
type Counter = U64<BigEndian>;
#[derive(Debug)]
pub(super) struct HistoryStore {
    path: PathBuf,
    env: Env,
    operations: Database<'static, IdKey, SerdeRmp<Record>>,
    chronology: Database<'static, Counter, IdKey>,
    contents: Database<'static, Bytes, SerdeRmp<Blob>>,
    references: Database<'static, Bytes, Counter>,
    pending: Database<'static, U8, SerdeRmp<Journal>>,
    sequence: Database<'static, U8, Counter>,
}
