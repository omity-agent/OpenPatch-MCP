use crate::operation::model::FileState;
use anyhow::Context as _;
use rusqlite::{Connection, OptionalExtension as _, params};
use smallvec::SmallVec;
const RAW_CODEC: i64 = 0;
const LZ4_CODEC: i64 = 1;
type Digest = [u8; blake3::OUT_LEN];
pub(super) struct Writer<'transaction, 'content> {
    connection: &'transaction Connection,
    seen: SmallVec<[(Digest, &'content str); 4]>,
}
impl<'transaction, 'content> Writer<'transaction, 'content> {
    pub(super) fn new(connection: &'transaction Connection) -> Self {
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
}
struct Encoded {
    digest: Digest,
    raw_len: i64,
    codec: i64,
    payload: Vec<u8>,
}
impl Encoded {
    fn new(contents: &str, digest: Digest) -> anyhow::Result<Self> {
        let bytes = contents.as_bytes();
        let raw_len = i64::try_from(bytes.len()).context("history content is too large")?;
        let mut payload = lz4_flex::block::compress(bytes);
        let codec = if payload.len() < bytes.len() {
            LZ4_CODEC
        } else {
            payload.clear();
            payload.extend_from_slice(bytes);
            RAW_CODEC
        };
        Ok(Self {
            digest,
            raw_len,
            codec,
            payload,
        })
    }
    fn insert(&self, connection: &Connection, contents: &str) -> anyhow::Result<()> {
        let inserted = connection
            .prepare_cached(
                "INSERT INTO history_blobs(digest, raw_len, codec, payload)
                 VALUES (?1, ?2, ?3, ?4) ON CONFLICT(digest) DO NOTHING",
            )?
            .execute(params![
                self.digest.as_slice(),
                self.raw_len,
                self.codec,
                self.payload
            ])?;
        if inserted == 0 {
            let existing = load(connection, self.digest)?;
            anyhow::ensure!(
                existing == contents,
                "BLAKE3 collision between stored operation history contents"
            );
        }
        Ok(())
    }
}
pub(super) struct Reader<'connection> {
    connection: &'connection Connection,
    seen: SmallVec<[(Digest, FileState); 4]>,
}
impl<'connection> Reader<'connection> {
    pub(super) fn new(connection: &'connection Connection) -> Self {
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
fn load(connection: &Connection, digest: Digest) -> anyhow::Result<String> {
    load_optional(connection, digest)?
        .ok_or_else(|| anyhow::anyhow!("operation history references missing content"))
}
fn load_optional(connection: &Connection, digest: Digest) -> anyhow::Result<Option<String>> {
    connection
        .prepare_cached("SELECT raw_len, codec, payload FROM history_blobs WHERE digest = ?1")?
        .query_row([digest.as_slice()], |row| {
            Ok(Stored {
                raw_len: row.get(0)?,
                codec: row.get(1)?,
                payload: row.get(2)?,
            })
        })
        .optional()?
        .map(|stored| stored.decode(digest))
        .transpose()
}
struct Stored {
    raw_len: i64,
    codec: i64,
    payload: Vec<u8>,
}
impl Stored {
    fn decode(self, expected_digest: Digest) -> anyhow::Result<String> {
        let raw_len = usize::try_from(self.raw_len)
            .context("invalid uncompressed content length in operation history")?;
        let bytes = match self.codec {
            RAW_CODEC => {
                anyhow::ensure!(
                    self.payload.len() == raw_len,
                    "raw operation history content has an invalid length"
                );
                self.payload
            }
            LZ4_CODEC => {
                let decompressed = lz4_flex::block::decompress(&self.payload, raw_len)
                    .context("failed to decompress operation history content")?;
                anyhow::ensure!(
                    decompressed.len() == raw_len,
                    "decompressed operation history content has an invalid length"
                );
                decompressed
            }
            codec => anyhow::bail!("unknown operation history content codec: {codec}"),
        };
        anyhow::ensure!(
            blake3::hash(&bytes).as_bytes() == &expected_digest,
            "operation history content digest mismatch"
        );
        String::from_utf8(bytes).context("operation history content is not valid UTF-8")
    }
}
pub(super) fn delete_unreferenced(connection: &Connection) -> anyhow::Result<()> {
    connection.execute(
        "DELETE FROM history_blobs
         WHERE NOT EXISTS (
             SELECT 1 FROM operation_files WHERE before_digest = history_blobs.digest
         ) AND NOT EXISTS (
             SELECT 1 FROM operation_files WHERE after_digest = history_blobs.digest
         )",
        [],
    )?;
    Ok(())
}
