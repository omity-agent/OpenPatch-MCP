mod boundary;
mod parse;
use smallvec::SmallVec;
use std::path::PathBuf;
pub(crate) const BEGIN_PATCH_MARKER: &str = "*** Begin Patch";
pub(crate) const END_PATCH_MARKER: &str = "*** End Patch";
pub(crate) const ADD_FILE_MARKER: &str = "*** Add File: ";
pub(crate) const DELETE_FILE_MARKER: &str = "*** Delete File: ";
pub(crate) const UPDATE_FILE_MARKER: &str = "*** Update File: ";
pub(crate) const MOVE_TO_MARKER: &str = "*** Move to: ";
pub(crate) const EOF_MARKER: &str = "*** End of File";
pub(crate) const CHANGE_CONTEXT_MARKER: &str = "@@ ";
pub(crate) const EMPTY_CHANGE_CONTEXT_MARKER: &str = "@@";
pub fn parse_patch(patch: &str) -> Result<Vec<FileHunk>, ParseFailure> {
    parse::parse_patch(patch)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileHunk {
    Add {
        path: PathBuf,
        contents: String,
        line_count: usize,
        character_count: usize,
    },
    Delete {
        path: PathBuf,
    },
    Update {
        path: PathBuf,
        move_path: Option<PathBuf>,
        chunks: Vec<UpdateChunk>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateChunk {
    pub change_context: Option<String>,
    pub old_lines: ChunkLines,
    pub new_lines: ChunkLines,
    pub is_end_of_file: bool,
}
pub(crate) type ChunkLines = SmallVec<[String; 4]>;
#[derive(Debug, Clone, PartialEq, Eq, thiserror :: Error)]
pub enum ParseFailure {
    #[error("Invalid patch: {0}")]
    Patch(String),
    #[error("Invalid patch hunk on line {line_number}: {message}")]
    Hunk { line_number: usize, message: String },
}
impl ParseFailure {
    #[must_use]
    pub(crate) fn patch(message: &str) -> Self {
        Self::Patch(message.to_owned())
    }
    #[must_use]
    pub(crate) fn hunk(line_number: usize, message: &str) -> Self {
        Self::Hunk {
            line_number,
            message: message.to_owned(),
        }
    }
}
