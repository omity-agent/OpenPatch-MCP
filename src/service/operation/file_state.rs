use alloc::sync::Arc;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileContents(Arc<String>);
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
    pub(crate) fn contents(&self) -> Option<&str> {
        match self {
            &Self::Missing => None,
            Self::Present(contents) => Some(contents.as_str()),
        }
    }
}
