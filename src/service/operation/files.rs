use super::model::{FileState, Mutation};
use atomic_write_file::AtomicWriteFile;
use smallvec::SmallVec;
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
pub(super) fn apply(mutation: &Mutation) -> anyhow::Result<()> {
    let states = mutation
        .changes
        .iter()
        .map(|change| &change.before)
        .collect::<SmallVec<[_; 2]>>();
    apply_from(mutation, &states)
}
pub(super) fn apply_observed(mutation: &Mutation, observed: &[FileState]) -> anyhow::Result<()> {
    ensure_state_count(mutation, observed)?;
    let states = observed.iter().collect::<SmallVec<[_; 2]>>();
    apply_from(mutation, &states)
}
pub(super) fn roll_back(mutation: &Mutation) -> anyhow::Result<()> {
    let states = mutation
        .changes
        .iter()
        .map(|change| &change.before)
        .collect::<SmallVec<[_; 2]>>();
    restore_states(mutation, &states, mutation.changes.len())
}
pub(super) fn roll_back_observed(
    mutation: &Mutation,
    observed: &[FileState],
) -> anyhow::Result<()> {
    ensure_state_count(mutation, observed)?;
    let states = observed.iter().collect::<SmallVec<[_; 2]>>();
    restore_states(mutation, &states, mutation.changes.len())
}
fn apply_from(mutation: &Mutation, states: &[&FileState]) -> anyhow::Result<()> {
    verify_states(mutation, states)?;
    for (index, (change, original)) in mutation.changes.iter().zip(states).enumerate() {
        if **original == change.after {
            continue;
        }
        if let Err(error) = write_state(&change.path, &change.after) {
            let rollback_result = restore_states(mutation, states, index + 1);
            return match rollback_result {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(anyhow::anyhow!(
                    "{error}; additionally failed to roll back file changes: {rollback_error}"
                )),
            };
        }
    }
    Ok(())
}
fn verify_states(mutation: &Mutation, states: &[&FileState]) -> anyhow::Result<()> {
    if mutation.changes.len() != states.len() {
        anyhow::bail!("file operation state count does not match its path count");
    }
    for (change, expected) in mutation.changes.iter().zip(states) {
        if !state_matches(
            &change.path,
            expected,
            "Failed to verify file before writing",
        )? {
            anyhow::bail!(
                "file changed concurrently before operation could be committed: {}",
                change.path.display()
            );
        }
    }
    Ok(())
}
fn state_matches(path: &Path, expected: &FileState, action: &str) -> anyhow::Result<bool> {
    let Some(expected_contents) = expected.contents() else {
        return match fs::metadata(path) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(anyhow::anyhow!("{action}: {error}")),
        };
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(anyhow::anyhow!("{action}: {error}")),
    };
    let expected_bytes = expected_contents.as_bytes();
    let mut compared_bytes = 0_usize;
    let mut reader = io::BufReader::new(file);
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| anyhow::anyhow!("{action}: {error}"))?;
        if available.is_empty() {
            return Ok(compared_bytes == expected_bytes.len());
        }
        let available_len = available.len();
        let next_compared_bytes = compared_bytes
            .checked_add(available_len)
            .ok_or_else(|| anyhow::anyhow!("{action}: file size overflowed"))?;
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
fn restore_states(
    mutation: &Mutation,
    states: &[&FileState],
    change_count: usize,
) -> anyhow::Result<()> {
    let mut errors = SmallVec::<[String; 2]>::new();
    for (change, original) in mutation.changes.iter().zip(states).take(change_count).rev() {
        if **original == change.after {
            continue;
        }
        if let Err(error) = write_state(&change.path, original) {
            errors.push(format!("{}: {error}", change.path.display()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(errors.join("; "));
    }
}
fn ensure_state_count(mutation: &Mutation, states: &[FileState]) -> anyhow::Result<()> {
    if mutation.changes.len() != states.len() {
        anyhow::bail!("file operation state count does not match its path count");
    }
    Ok(())
}
fn write_state(path: &Path, state: &FileState) -> anyhow::Result<()> {
    state
        .contents()
        .map_or_else(|| remove_file(path), |contents| write_file(path, contents))
}
fn write_file(path: &Path, contents: &str) -> anyhow::Result<()> {
    match write_file_atomically(path, contents) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| {
                    anyhow::anyhow!("Failed to create parent directories: {source}")
                })?;
            }
            write_file_atomically(path, contents)
                .map_err(|source| anyhow::anyhow!("Failed to write file: {source}"))
        }
        Err(error) => Err(anyhow::anyhow!("Failed to write file: {error}")),
    }
}
fn write_file_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(contents.as_bytes())?;
    file.commit()
}
fn remove_file(path: &Path) -> anyhow::Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => anyhow::bail!("path is a directory"),
        Ok(_) => {
            fs::remove_file(path).map_err(|error| anyhow::anyhow!("Failed to delete file: {error}"))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(anyhow::anyhow!("Failed to inspect file: {error}")),
    }
}
