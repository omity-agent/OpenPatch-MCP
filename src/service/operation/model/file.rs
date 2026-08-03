use alloc::sync::Arc;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileContents(
    #[expect(
        clippy::rc_buffer,
        reason = "Arc<String> takes ownership of read_to_string buffers without copying their text"
    )]
    Arc<String>,
);
impl FileContents {
    fn new(contents: String) -> Self {
        Self(Arc::new(contents))
    }
    fn share(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl core::ops::Deref for FileContents {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl AsRef<str> for FileContents {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl From<String> for FileContents {
    fn from(contents: String) -> Self {
        Self::new(contents)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileState {
    Missing,
    Present(FileContents),
}
impl FileState {
    pub(crate) fn present(contents: String) -> Self {
        Self::Present(FileContents::new(contents))
    }
    pub(crate) fn share(contents: &FileContents) -> Self {
        Self::Present(contents.share())
    }
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching the borrowed enum keeps the returned contents borrowed"
    )]
    pub(crate) fn contents(&self) -> Option<&str> {
        match self {
            &Self::Missing => None,
            Self::Present(contents) => Some(contents.as_str()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{Arc, FileState};
    #[test]
    fn shared_states_reuse_the_same_text_allocation() {
        let state = FileState::present("large contents".repeat(1_024));
        let FileState::Present(contents) = state else {
            panic!("constructed state must be present");
        };
        let shared = FileState::share(&contents);
        let FileState::Present(shared_contents) = shared else {
            panic!("shared state must be present");
        };
        assert!(Arc::ptr_eq(&contents.0, &shared_contents.0));
    }
}
