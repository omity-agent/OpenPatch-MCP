use anyhow::Context as _;
use diesel::{
    OptionalExtension as _, RunQueryDsl as _,
    sql_types::{BigInt, Binary},
    sqlite::SqliteConnection,
};
const RAW_CODEC: i64 = 0;
const LZ4_CODEC: i64 = 1;
pub(super) type Digest = [u8; blake3::OUT_LEN];
pub(super) struct Encoded {
    digest: Digest,
    raw_len: i64,
    codec: i64,
    payload: Vec<u8>,
}
impl Encoded {
    pub(super) fn new(contents: &str, digest: Digest) -> anyhow::Result<Self> {
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
    pub(super) fn insert(
        &self,
        connection: &mut SqliteConnection,
        contents: &str,
    ) -> anyhow::Result<()> {
        let inserted = diesel::sql_query(
            "INSERT INTO history_blobs(digest, raw_len, codec, payload)
             VALUES (?, ?, ?, ?) ON CONFLICT(digest) DO NOTHING",
        )
        .bind::<Binary, _>(self.digest.as_slice())
        .bind::<BigInt, _>(self.raw_len)
        .bind::<BigInt, _>(self.codec)
        .bind::<Binary, _>(self.payload.as_slice())
        .execute(connection)?;
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
pub(super) fn load(connection: &mut SqliteConnection, digest: Digest) -> anyhow::Result<String> {
    load_optional(connection, digest)?
        .ok_or_else(|| anyhow::anyhow!("operation history references missing content"))
}
pub(super) fn load_optional(
    connection: &mut SqliteConnection,
    digest: Digest,
) -> anyhow::Result<Option<String>> {
    diesel::sql_query("SELECT raw_len, codec, payload FROM history_blobs WHERE digest = ?")
        .bind::<Binary, _>(digest.as_slice())
        .get_result::<super::rows::StoredContent>(connection)
        .optional()?
        .map(|stored| stored.decode(digest))
        .transpose()
}
impl super::rows::StoredContent {
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
