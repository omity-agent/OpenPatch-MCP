use super::super::{model::FileState, store::Transition};
use anyhow::Context as _;
use atomic_write_file::AtomicWriteFile;
use std::{
    fs,
    io::{self, BufRead as _, Write as _},
    path::Path,
};
pub(crate) fn snapshot(path: &Path, action: &str) -> anyhow::Result<FileState> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(FileState::present(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(FileState::Missing),
        Err(error) => Err(anyhow::anyhow!("{action}: {error}")),
    }
}
pub(crate) fn verify(transitions: &[Transition]) -> anyhow::Result<()> {
    for transition in transitions {
        anyhow::ensure!(
            state_matches(&transition.path, &transition.before)?,
            "file changed concurrently before operation could be committed: {}",
            transition.path.display()
        );
    }
    Ok(())
}
pub(crate) fn publish(transitions: &[Transition]) -> anyhow::Result<()> {
    let pending = transitions
        .iter()
        .map(needs_write)
        .collect::<anyhow::Result<Vec<_>>>()?;
    for (transition, changed) in transitions.iter().zip(pending) {
        if changed && needs_write(transition)? {
            write_state(&transition.path, &transition.after).with_context(|| {
                format!("failed to publish file: {}", transition.path.display())
            })?;
        }
    }
    Ok(())
}
fn needs_write(transition: &Transition) -> anyhow::Result<bool> {
    if state_matches(&transition.path, &transition.before)? {
        return Ok(transition.before != transition.after);
    }
    anyhow::ensure!(
        state_matches(&transition.path, &transition.after)?,
        "file changed outside the recorded operation: {}",
        transition.path.display()
    );
    Ok(false)
}
fn state_matches(path: &Path, expected: &FileState) -> anyhow::Result<bool> {
    let Some(expected_contents) = expected.contents() else {
        return match fs::metadata(path) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(error.into()),
        };
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let expected_bytes = expected_contents.as_bytes();
    if file.metadata()?.len() != u64::try_from(expected_bytes.len())? {
        return Ok(false);
    }
    let mut compared_bytes = 0_usize;
    let mut reader = io::BufReader::new(file);
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(compared_bytes == expected_bytes.len());
        }
        let available_len = available.len();
        let next_compared_bytes = compared_bytes
            .checked_add(available_len)
            .ok_or_else(|| anyhow::anyhow!("file size overflowed"))?;
        let Some(expected_chunk) = expected_bytes.get(compared_bytes..next_compared_bytes) else {
            return Ok(false);
        };
        if expected_chunk != available {
            return Ok(false);
        }
        compared_bytes = next_compared_bytes;
        reader.consume(available_len);
    }
}
pub(crate) fn write_state(path: &Path, state: &FileState) -> anyhow::Result<()> {
    if let Some(contents) = state.contents() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = AtomicWriteFile::open(path)?;
        file.write_all(contents.as_bytes())?;
        file.commit()?;
    } else {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
