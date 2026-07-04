use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub(crate) struct FileWriter<'cwd> {
    cwd: &'cwd Path,
}
impl<'cwd> FileWriter<'cwd> {
    pub(crate) const fn new(cwd: &'cwd Path) -> Self {
        Self { cwd }
    }
    pub(crate) fn read_file_to_update(&self, path: &Path) -> anyhow::Result<String> {
        let source = self.resolve(path)?;
        fs::read_to_string(&source)
            .map_err(|error| io_context(&error, "Failed to read file to update", &source))
    }
    pub(crate) fn write_file(&self, path: &Path, contents: String) -> anyhow::Result<()> {
        let target = self.resolve(path)?;
        fs::write(&target, contents)
            .map_err(|error| io_context(&error, "Failed to write file", &target))
    }
    pub(crate) fn write_with_parent_retry(
        &self,
        path: &Path,
        contents: String,
    ) -> anyhow::Result<()> {
        let target = self.resolve(path)?;
        match fs::write(&target, contents.as_bytes()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|source| {
                        io_context(&source, "Failed to create parent directories for", &target)
                    })?;
                }
                fs::write(&target, contents)
                    .map_err(|source| io_context(&source, "Failed to write file", &target))
            }
            Err(error) => Err(io_context(&error, "Failed to write file", &target)),
        }
    }
    pub(crate) fn delete_file(&self, path: &Path) -> anyhow::Result<()> {
        let target = self.resolve(path)?;
        ensure_not_directory(&target)?;
        fs::remove_file(&target)
            .map_err(|error| io_context(&error, "Failed to delete file", &target))
    }
    pub(crate) fn delete_original(&self, path: &Path) -> anyhow::Result<()> {
        let source = self.resolve(path)?;
        ensure_not_directory(&source)?;
        fs::remove_file(&source)
            .map_err(|error| io_context(&error, "Failed to remove original", &source))
    }
    pub(crate) fn resolve(&self, path: &Path) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(
            path.is_relative(),
            "patch paths must be relative: {}",
            path.display()
        );
        Ok(self.cwd.join(path))
    }
}
fn ensure_not_directory(path: &Path) -> anyhow::Result<()> {
    if fs::metadata(path)
        .map_err(|error| io_context(&error, "Failed to inspect file", path))?
        .is_dir()
    {
        anyhow::bail!("{} is a directory", path.display());
    }
    Ok(())
}
fn io_context(error: &io::Error, action: &str, path: &Path) -> anyhow::Error {
    anyhow::anyhow!("{action} {}: {error}", path.display())
}
