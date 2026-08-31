use super::codec::{Digest, Encoded, load, load_optional};
use crate::operation::model::FileState;
use diesel::{RunQueryDsl as _, sqlite::SqliteConnection};
use smallvec::SmallVec;
pub(super) struct Writer<'transaction, 'content> {
    connection: &'transaction mut SqliteConnection,
    seen: SmallVec<[(Digest, &'content str); 4]>,
}
impl<'transaction, 'content> Writer<'transaction, 'content> {
    pub(super) fn new(connection: &'transaction mut SqliteConnection) -> Self {
        Self {
            connection,
            seen: SmallVec::new(),
        }
    }
    pub(super) fn store(&mut self, state: &'content FileState) -> anyhow::Result<Option<Digest>> {
        let Some(stored_contents) = state.contents() else {
            return Ok(None);
        };
        let contents: &str = stored_contents;
        let digest = *blake3::hash(contents.as_bytes()).as_bytes();
        if let Some(entry) = self.seen.iter().find(|entry| entry.0 == digest) {
            anyhow::ensure!(
                entry.1 == contents,
                "BLAKE3 collision between operation history contents"
            );
            return Ok(Some(digest));
        }
        if let Some(existing) = load_optional(self.connection, digest)? {
            anyhow::ensure!(
                existing == contents,
                "BLAKE3 collision between stored operation history contents"
            );
            self.seen.push((digest, contents));
            return Ok(Some(digest));
        }
        let encoded = Encoded::new(contents, digest)?;
        encoded.insert(self.connection, contents)?;
        self.seen.push((digest, contents));
        Ok(Some(digest))
    }
    pub(super) const fn connection(&mut self) -> &mut SqliteConnection {
        self.connection
    }
}
pub(super) struct Reader<'connection> {
    connection: &'connection mut SqliteConnection,
    seen: SmallVec<[(Digest, FileState); 4]>,
}
impl<'connection> Reader<'connection> {
    pub(super) fn new(connection: &'connection mut SqliteConnection) -> Self {
        Self {
            connection,
            seen: SmallVec::new(),
        }
    }
    pub(super) fn load(&mut self, stored_digest: Option<Vec<u8>>) -> anyhow::Result<FileState> {
        let Some(bytes) = stored_digest else {
            return Ok(FileState::Missing);
        };
        let digest: Digest = bytes.try_into().map_err(|invalid: Vec<u8>| {
            anyhow::anyhow!(
                "invalid content digest length in operation history: {}",
                invalid.len()
            )
        })?;
        if let Some(entry) = self.seen.iter().find(|entry| entry.0 == digest) {
            return Ok(entry.1.clone());
        }
        let state = FileState::present(load(self.connection, digest)?);
        self.seen.push((digest, state.clone()));
        Ok(state)
    }
}
pub(super) fn delete_unreferenced(connection: &mut SqliteConnection) -> anyhow::Result<()> {
    diesel::sql_query(
        "DELETE FROM history_blobs
         WHERE NOT EXISTS (
             SELECT 1 FROM operation_files WHERE before_digest = history_blobs.digest
         ) AND NOT EXISTS (
             SELECT 1 FROM operation_files WHERE after_digest = history_blobs.digest
         )",
    )
    .execute(connection)?;
    Ok(())
}
