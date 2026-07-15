use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub(crate) struct FileWriter;
impl FileWriter {
    pub(crate) fn read_file_to_update(path: &Path) -> anyhow::Result<(PathBuf, String)> {
        let source = path.to_path_buf();
        let contents = fs::read_to_string(&source)
            .map_err(|error| io_context(&error, "Failed to read file to update"))?;
        Ok((source, contents))
    }
    pub(crate) fn read_file_to_delete(path: &Path) -> anyhow::Result<(PathBuf, String)> {
        let source = path.to_path_buf();
        ensure_not_directory(&source)
            .map_err(|error| io_context(&error, "Failed to delete file"))?;
        let contents = fs::read_to_string(&source)
            .map_err(|error| io_context(&error, "Failed to read file to delete"))?;
        Ok((source, contents))
    }
    pub(crate) fn write_resolved_file(target: &Path, contents: String) -> anyhow::Result<()> {
        write_resolved_file(target, contents)
    }
    pub(crate) fn write_with_parent_retry(path: &Path, contents: String) -> anyhow::Result<()> {
        let target = path.to_path_buf();
        write_resolved_with_parent_retry(&target, contents)
    }
    pub(crate) fn delete_resolved_file(target: &Path) -> anyhow::Result<()> {
        delete_resolved_file(target, "Failed to delete file")
    }
    pub(crate) fn delete_resolved_original(source: &Path) -> anyhow::Result<()> {
        delete_resolved_file(source, "Failed to remove original")
    }
}
fn write_resolved_file(target: &Path, contents: String) -> anyhow::Result<()> {
    fs::write(target, contents).map_err(|error| io_context(&error, "Failed to write file"))
}
fn write_resolved_with_parent_retry(target: &Path, contents: String) -> anyhow::Result<()> {
    match fs::write(target, contents.as_bytes()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|source| io_context(&source, "Failed to create parent directories"))?;
            }
            fs::write(target, contents)
                .map_err(|source| io_context(&source, "Failed to write file"))
        }
        Err(error) => Err(io_context(&error, "Failed to write file")),
    }
}
fn delete_resolved_file(target: &Path, action: &str) -> anyhow::Result<()> {
    ensure_not_directory(target).map_err(|error| io_context(&error, action))?;
    fs::remove_file(target).map_err(|error| io_context(&error, action))
}
fn ensure_not_directory(path: &Path) -> io::Result<()> {
    if fs::metadata(path)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is a directory",
        ));
    }
    Ok(())
}
fn io_context(error: &io::Error, action: &str) -> anyhow::Error {
    anyhow::anyhow!("{action}: {error}")
}
