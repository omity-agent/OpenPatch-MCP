mod content;
mod fs_ops;
mod summary;
use crate::{
    parser::{FileHunk, parse_patch},
    patch::{content::derive_new_contents, fs_ops::FileWriter},
};
use std::path::Path;
use summary::{FileChange, FileStats, Summary};
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
            let after = FileStats::from_contents(&contents);
            writer.write_with_parent_retry(&path, contents)?;
            summary
                .added
                .push(FileChange::new(path, FileStats::empty(), after));
        }
        FileHunk::Delete { path } => {
            let original_contents = writer.read_file_to_delete(&path)?;
            let before = FileStats::from_contents(&original_contents);
            writer.delete_file(&path)?;
            summary
                .deleted
                .push(FileChange::new(path, before, FileStats::empty()));
        }
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => {
            let original_contents = writer.read_file_to_update(&path)?;
            let before = FileStats::from_contents(&original_contents);
            let source = writer.resolve(&path);
            let derived = derive_new_contents(&source, &original_contents, &chunks);
            let after = FileStats::from_contents(&derived.contents);
            summary.errors.extend(derived.errors);
            if let Some(destination_path) = move_path {
                if chunks.is_empty() || derived.applied_chunks > 0 {
                    writer.write_with_parent_retry(&destination_path, derived.contents)?;
                    writer.delete_original(&path)?;
                    summary
                        .modified
                        .push(FileChange::new(destination_path, before, after));
                }
            } else if derived.applied_chunks > 0 {
                writer.write_file(&path, derived.contents)?;
                summary.modified.push(FileChange::new(path, before, after));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::apply_patch_text;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    #[test]
    fn failed_chunk_does_not_stop_following_chunks_in_same_file() {
        let directory = unique_temp_directory().unwrap();
        let target_path = directory.join("target.txt");
        fs::write(&target_path, "one\ntwo\nthree\n").unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Update File: target.txt",
            "@@",
            "-one",
            "+1",
            "@@",
            "-missing",
            "+changed",
            "@@",
            "-three",
            "+3",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let result = apply_patch_text(&patch, &directory);
        assert!(result.stdout.contains("M target.txt"));
        assert!(result.stderr.contains("Failed to find expected lines"));
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "1\ntwo\n3\n");
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn output_includes_file_stats_before_and_after_changes() {
        let directory = unique_temp_directory().unwrap();
        let target_path = directory.join("target.txt");
        let obsolete_path = directory.join("obsolete.txt");
        fs::write(&target_path, "old\n").unwrap();
        fs::write(&obsolete_path, "bye\n").unwrap();
        let patch = [
            "*** Begin Patch",
            "*** Add File: hello.txt",
            "+hello",
            "+world",
            "*** Update File: target.txt",
            "@@",
            "-old",
            "+new",
            "*** Delete File: obsolete.txt",
            "*** End Patch",
            "",
        ]
        .join("\n");
        let result = apply_patch_text(&patch, &directory);
        assert!(result.stderr.is_empty(), "{}", result.stderr);
        assert!(
            result
                .stdout
                .contains("A hello.txt (before: 0 lines, 0 chars; after: 2 lines, 12 chars)")
        );
        assert!(
            result
                .stdout
                .contains("M target.txt (before: 1 lines, 4 chars; after: 1 lines, 4 chars)")
        );
        assert!(
            result
                .stdout
                .contains("D obsolete.txt (before: 1 lines, 4 chars; after: 0 lines, 0 chars)")
        );
        fs::remove_dir_all(directory).unwrap();
    }
    fn unique_temp_directory() -> anyhow::Result<PathBuf> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory =
            std::env::temp_dir().join(format!("apply-patch-mcp-{}-{suffix}", std::process::id()));
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }
}
