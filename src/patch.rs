mod content;
mod fs_ops;
use crate::{
    parser::{FileHunk, parse_patch},
    patch::{content::derive_new_contents, fs_ops::FileWriter},
};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub stdout: String,
    pub stderr: String,
}
pub fn apply_patch_text(patch: &str, cwd: &Path) -> ApplyResult {
    match apply_patch_text_inner(patch, cwd) {
        Ok(summary) => ApplyResult {
            stdout: summary.render(),
            stderr: summary.render_errors(),
        },
        Err(error) => ApplyResult {
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}
fn apply_patch_text_inner(patch: &str, cwd: &Path) -> anyhow::Result<Summary> {
    let hunks = parse_patch(patch)?;
    anyhow::ensure!(!hunks.is_empty(), "No files were modified.");
    let writer = FileWriter::new(cwd);
    let mut summary = Summary::default();
    for hunk in hunks {
        if let Err(error) = apply_hunk(&writer, hunk, &mut summary) {
            summary.errors.push(error.to_string());
        }
    }
    Ok(summary)
}
fn apply_hunk(
    writer: &FileWriter<'_>,
    hunk: FileHunk,
    summary: &mut Summary,
) -> anyhow::Result<()> {
    match hunk {
        FileHunk::Add { path, contents } => {
            writer.write_with_parent_retry(&path, contents)?;
            summary.added.push(path);
        }
        FileHunk::Delete { path } => {
            writer.delete_file(&path)?;
            summary.deleted.push(path);
        }
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => {
            let original_contents = writer.read_file_to_update(&path)?;
            let source = writer.resolve(&path)?;
            let new_contents = derive_new_contents(&source, &original_contents, &chunks)?;
            if let Some(destination_path) = move_path {
                writer.write_with_parent_retry(&destination_path, new_contents)?;
                writer.delete_original(&path)?;
                summary.modified.push(destination_path);
            } else {
                writer.write_file(&path, new_contents)?;
                summary.modified.push(path);
            }
        }
    }
    Ok(())
}
#[derive(Debug, Default)]
struct Summary {
    added: Vec<PathBuf>,
    modified: Vec<PathBuf>,
    deleted: Vec<PathBuf>,
    errors: Vec<String>,
}
impl Summary {
    fn render(&self) -> String {
        let mut output = if self.errors.is_empty() {
            String::from("Success. Updated the following files:\n")
        } else {
            String::from("Updated the following files:\n")
        };
        for path in &self.added {
            push_path_line(&mut output, 'A', path);
        }
        for path in &self.modified {
            push_path_line(&mut output, 'M', path);
        }
        for path in &self.deleted {
            push_path_line(&mut output, 'D', path);
        }
        output
    }
    fn render_errors(&self) -> String {
        let mut output = String::new();
        for error in &self.errors {
            output.push_str(error);
            output.push('\n');
        }
        output
    }
}
fn push_path_line(output: &mut String, marker: char, path: &Path) {
    output.push(marker);
    output.push(' ');
    output.push_str(&path.display().to_string());
    output.push('\n');
}
