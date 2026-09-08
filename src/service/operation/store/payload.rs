use super::HistoryStore;
use crate::operation::model::FileState;
use anyhow::Context as _;
use heed::{RoTxn, RwTxn};
use serde::{Deserialize, Serialize};
pub(super) type Digest = [u8; blake3::OUT_LEN];
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Blob {
    raw_len: usize,
    compressed: bool,
    #[serde(with = "serde_bytes")]
    payload: Vec<u8>,
}
impl Blob {
    fn encode(contents: &str) -> Self {
        let bytes = contents.as_bytes();
        let compressed = lz4_flex::block::compress(bytes);
        if compressed.len() < bytes.len() {
            Self {
                raw_len: bytes.len(),
                compressed: true,
                payload: compressed,
            }
        } else {
            Self {
                raw_len: bytes.len(),
                compressed: false,
                payload: bytes.to_vec(),
            }
        }
    }
    fn decode(self, digest: Digest) -> anyhow::Result<String> {
        let bytes = if self.compressed {
            lz4_flex::block::decompress(&self.payload, self.raw_len)
                .context("failed to decompress operation history content")?
        } else {
            self.payload
        };
        anyhow::ensure!(
            bytes.len() == self.raw_len,
            "invalid operation history content length"
        );
        anyhow::ensure!(
            blake3::hash(&bytes).as_bytes() == &digest,
            "operation history content digest mismatch"
        );
        String::from_utf8(bytes).context("operation history content is not valid UTF-8")
    }
}
impl HistoryStore {
    pub(super) fn read_content(
        &self,
        transaction: &RoTxn<'_>,
        digest: Option<Digest>,
    ) -> anyhow::Result<FileState> {
        let Some(key) = digest else {
            return Ok(FileState::Missing);
        };
        let blob = self
            .contents
            .get(transaction, &key)?
            .context("operation history references missing content")?;
        Ok(FileState::present(blob.decode(key)?))
    }
    pub(super) fn retain_content(
        &self,
        transaction: &mut RwTxn<'_>,
        state: &FileState,
    ) -> anyhow::Result<Option<Digest>> {
        let Some(contents) = state.contents() else {
            return Ok(None);
        };
        let digest = *blake3::hash(contents.as_bytes()).as_bytes();
        let count = if let Some(existing) = self.references.get(transaction, &digest)? {
            existing
                .checked_add(1)
                .context("content reference count overflow")?
        } else {
            self.contents
                .put(transaction, &digest, &Blob::encode(contents))?;
            1
        };
        self.references.put(transaction, &digest, &count)?;
        Ok(Some(digest))
    }
    pub(super) fn release_content(
        &self,
        transaction: &mut RwTxn<'_>,
        digest: Option<Digest>,
    ) -> anyhow::Result<()> {
        let Some(key) = digest else {
            return Ok(());
        };
        let count = self
            .references
            .get(transaction, &key)?
            .context("missing content reference count")?;
        let remaining = count
            .checked_sub(1)
            .context("content reference count underflow")?;
        if remaining == 0 {
            anyhow::ensure!(
                self.contents.delete(transaction, &key)?,
                "missing referenced content"
            );
            self.references.delete(transaction, &key)?;
        } else {
            self.references.put(transaction, &key, &remaining)?;
        }
        Ok(())
    }
}
