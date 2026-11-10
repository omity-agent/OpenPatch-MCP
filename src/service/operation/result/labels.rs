use super::OperationOutput;
use std::path::{Component, Path, PathBuf};
pub(super) struct PathLabels {
    base: PathBuf,
}
impl PathLabels {
    pub(super) fn new(output: &OperationOutput) -> Self {
        let reported = output
            .successes
            .iter()
            .map(|success| Path::new(&success.path))
            .chain(output.failures.iter().filter_map(|failure| {
                failure
                    .operation
                    .as_ref()
                    .map(|operation| Path::new(&operation.path))
            }));
        let diagnostic = output
            .failures
            .iter()
            .flat_map(|failure| failure.reason.paths());
        let paths = if reported.clone().next().is_some() {
            reported.collect::<Vec<_>>()
        } else {
            diagnostic.collect::<Vec<_>>()
        };
        let base = paths
            .into_iter()
            .map(|path| path.parent().unwrap_or_else(|| Path::new("")).to_path_buf())
            .reduce(|common, parent| {
                common
                    .components()
                    .zip(parent.components())
                    .take_while(|&(left, right)| left == right)
                    .map(|(component, _)| component)
                    .collect()
            })
            .unwrap_or_default();
        Self { base }
    }
    pub(super) fn display(&self, path: &Path) -> String {
        let relative = path
            .strip_prefix(&self.base)
            .unwrap_or_else(|_| path.file_name().map_or_else(|| Path::new("."), Path::new));
        let parts = relative
            .components()
            .filter_map(|component| match component {
                Component::Prefix(prefix) => Some(format!(
                    "[{}]",
                    prefix
                        .as_os_str()
                        .to_string_lossy()
                        .trim_start_matches(['\\', '/'])
                        .replace('\\', "/")
                )),
                Component::RootDir | Component::CurDir => None,
                Component::ParentDir => Some(String::from("..")),
                Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            })
            .collect::<Vec<_>>();
        if parts.is_empty() {
            String::from(".")
        } else {
            parts.join("/")
        }
    }
}
