use super::model::{FileState, Mutation};
use std::{fs, io, path::Path};
pub(crate) fn snapshot(path: &Path, action: &str) -> anyhow::Result<FileState> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(FileState::Present(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(FileState::Missing),
        Err(error) => Err(anyhow::anyhow!("{action}: {error}")),
    }
}
pub(super) fn apply(mutation: &Mutation) -> anyhow::Result<()> {
    verify_before(mutation)?;
    for (index, change) in mutation.changes.iter().enumerate() {
        if let Err(error) = write_state(&change.path, &change.after) {
            let rollback_result = restore_before(mutation, index);
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
pub(super) fn roll_back(mutation: &Mutation) -> anyhow::Result<()> {
    restore_before(mutation, mutation.changes.len().saturating_sub(1))
}
fn verify_before(mutation: &Mutation) -> anyhow::Result<()> {
    for change in &mutation.changes {
        let current = snapshot(&change.path, "Failed to verify file before writing")?;
        if current != change.before {
            anyhow::bail!(
                "file changed concurrently before operation could be committed: {}",
                change.path.display()
            );
        }
    }
    Ok(())
}
fn restore_before(mutation: &Mutation, last_index: usize) -> anyhow::Result<()> {
    let mut errors = Vec::new();
    for change in mutation
        .changes
        .iter()
        .take(last_index.saturating_add(1))
        .rev()
    {
        if let Err(error) = write_state(&change.path, &change.before) {
            errors.push(format!("{}: {error}", change.path.display()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(errors.join("; "));
    }
}
fn write_state(path: &Path, state: &FileState) -> anyhow::Result<()> {
    state
        .contents()
        .map_or_else(|| remove_file(path), |contents| write_file(path, contents))
}
fn write_file(path: &Path, contents: &str) -> anyhow::Result<()> {
    match fs::write(path, contents.as_bytes()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| {
                    anyhow::anyhow!("Failed to create parent directories: {source}")
                })?;
            }
            fs::write(path, contents.as_bytes())
                .map_err(|source| anyhow::anyhow!("Failed to write file: {source}"))
        }
        Err(error) => Err(anyhow::anyhow!("Failed to write file: {error}")),
    }
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
