use core::fmt::Write as _;
use smallvec::SmallVec;
use std::path::PathBuf;
type FileChanges = SmallVec<[FileChange; 1]>;
#[derive(Debug, Default)]
pub(crate) struct Summary {
    pub(crate) added: FileChanges,
    pub(crate) modified: FileChanges,
    pub(crate) deleted: FileChanges,
    pub(crate) errors: Vec<String>,
}
impl Summary {
    pub(crate) fn render(&self) -> String {
        let mut output = String::with_capacity(self.render_capacity());
        if self.errors.is_empty() {
            output.push_str("Success. Updated the following files:\n");
        } else {
            output.push_str("Updated the following files:\n");
        }
        for change in &self.added {
            push_change_line(&mut output, 'A', change);
        }
        for change in &self.modified {
            push_change_line(&mut output, 'M', change);
        }
        for change in &self.deleted {
            push_change_line(&mut output, 'D', change);
        }
        output
    }
    pub(crate) fn render_errors(&self) -> String {
        let mut output = String::with_capacity(
            self.errors.iter().map(String::len).sum::<usize>() + self.errors.len(),
        );
        for error in &self.errors {
            output.push_str(error);
            output.push('\n');
        }
        output
    }
    fn render_capacity(&self) -> usize {
        let header = if self.errors.is_empty() {
            "Success. Updated the following files:\n".len()
        } else {
            "Updated the following files:\n".len()
        };
        header + (self.added.len() + self.modified.len() + self.deleted.len()) * 96
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileChange {
    path: PathBuf,
    before: FileStats,
    after: FileStats,
}
impl FileChange {
    pub(crate) const fn new(path: PathBuf, before: FileStats, after: FileStats) -> Self {
        Self {
            path,
            before,
            after,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStats {
    line_count: usize,
    character_count: usize,
}
impl FileStats {
    pub(crate) const fn from_counts(line_count: usize, character_count: usize) -> Self {
        Self {
            line_count,
            character_count,
        }
    }
    pub(crate) const fn empty() -> Self {
        Self {
            line_count: 0,
            character_count: 0,
        }
    }
    pub(crate) fn from_contents(contents: &str) -> Self {
        if contents.is_empty() {
            return Self::empty();
        }
        if contents.is_ascii() {
            return Self::from_ascii_contents(contents);
        }
        let newline_count = bytecount::count(contents.as_bytes(), b'\n');
        let character_count = bytecount::num_chars(contents.as_bytes());
        Self {
            line_count: newline_count + usize::from(!contents.ends_with('\n')),
            character_count,
        }
    }
    fn from_ascii_contents(contents: &str) -> Self {
        let newline_count = bytecount::count(contents.as_bytes(), b'\n');
        Self {
            line_count: newline_count + usize::from(!contents.ends_with('\n')),
            character_count: contents.len(),
        }
    }
}
fn push_change_line(output: &mut String, marker: char, change: &FileChange) {
    if let Err(error) = write!(output, "{marker} {} (before: ", change.path.display()) {
        panic!("writing to String failed: {error}");
    }
    push_stats(output, change.before);
    output.push_str("; after: ");
    push_stats(output, change.after);
    output.push_str(")\n");
}
fn push_stats(output: &mut String, stats: FileStats) {
    let mut buffer = itoa::Buffer::new();
    output.push_str(buffer.format(stats.line_count));
    output.push_str(" lines, ");
    output.push_str(buffer.format(stats.character_count));
    output.push_str(" chars");
}
