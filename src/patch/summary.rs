use std::path::PathBuf;
#[derive(Debug, Default)]
pub(crate) struct Summary {
    pub(crate) added: Vec<FileChange>,
    pub(crate) modified: Vec<FileChange>,
    pub(crate) deleted: Vec<FileChange>,
    pub(crate) errors: Vec<String>,
}
impl Summary {
    pub(crate) fn render(&self) -> String {
        let mut output = if self.errors.is_empty() {
            String::from("Success. Updated the following files:\n")
        } else {
            String::from("Updated the following files:\n")
        };
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
        let mut output = String::new();
        for error in &self.errors {
            output.push_str(error);
            output.push('\n');
        }
        output
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
    pub(crate) const fn empty() -> Self {
        Self {
            line_count: 0,
            character_count: 0,
        }
    }
    pub(crate) fn from_contents(contents: &str) -> Self {
        Self {
            line_count: line_count(contents),
            character_count: contents.chars().count(),
        }
    }
}
fn line_count(contents: &str) -> usize {
    if contents.is_empty() {
        return 0;
    }
    let line_count = contents.split('\n').count();
    if contents.ends_with('\n') {
        line_count - 1
    } else {
        line_count
    }
}
fn push_change_line(output: &mut String, marker: char, change: &FileChange) {
    output.push(marker);
    output.push(' ');
    output.push_str(&change.path.display().to_string());
    output.push_str(" (before: ");
    push_stats(output, change.before);
    output.push_str("; after: ");
    push_stats(output, change.after);
    output.push_str(")\n");
}
fn push_stats(output: &mut String, stats: FileStats) {
    output.push_str(&stats.line_count.to_string());
    output.push_str(" lines, ");
    output.push_str(&stats.character_count.to_string());
    output.push_str(" chars");
}
