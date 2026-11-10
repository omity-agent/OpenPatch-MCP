use super::labels::PathLabels;
use alloc::sync::Arc;
use sonic_rs::Serialize;
use std::path::{Path, PathBuf};
#[derive(Debug, thiserror::Error)]
#[error("{message}: {}", path.file_name().map_or_else(|| ".".into(), std::ffi::OsStr::to_string_lossy))]
pub(crate) struct PathContext {
    message: String,
    path: PathBuf,
    #[source]
    source: Option<Box<dyn core::error::Error + Send + Sync>>,
}
impl PathContext {
    pub(crate) fn new(message: &str, path: &Path) -> Self {
        Self {
            message: message.to_owned(),
            path: path.to_path_buf(),
            source: None,
        }
    }
    pub(crate) fn with_source(mut self, error: impl Into<anyhow::Error>) -> Self {
        self.source = Some(error.into().into_boxed_dyn_error());
        self
    }
}
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(in crate::service::operation) struct Diagnostic {
    pub(super) message: String,
    #[serde(skip)]
    error: Option<Arc<anyhow::Error>>,
}
impl Diagnostic {
    pub(super) fn paths(&self) -> impl Iterator<Item = &Path> {
        self.error.iter().flat_map(|error| {
            error
                .chain()
                .filter_map(|cause| cause.downcast_ref::<PathContext>())
                .map(|context| context.path.as_path())
        })
    }
    pub(super) fn finish(&mut self, labels: &PathLabels) {
        if let Some(error) = self.error.take() {
            self.message = error
                .chain()
                .map(|cause| {
                    cause.downcast_ref::<PathContext>().map_or_else(
                        || cause.to_string(),
                        |context| format!("{}: {}", context.message, labels.display(&context.path)),
                    )
                })
                .collect::<Vec<_>>()
                .join(": ");
        }
    }
}
impl From<String> for Diagnostic {
    fn from(message: String) -> Self {
        Self {
            message,
            error: None,
        }
    }
}
impl From<anyhow::Error> for Diagnostic {
    fn from(error: anyhow::Error) -> Self {
        Self::from(Arc::new(error))
    }
}
impl From<Arc<anyhow::Error>> for Diagnostic {
    fn from(error: Arc<anyhow::Error>) -> Self {
        Self {
            message: String::new(),
            error: Some(error),
        }
    }
}
