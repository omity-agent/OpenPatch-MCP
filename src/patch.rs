mod content;
mod fs_ops;
mod summary;
use crate::{
    parser::{FileHunk, parse_patch},
    patch::{content::derive_new_contents, fs_ops::FileWriter},
};
use summary::{FileChange, FileStats, Summary};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub stdout: String,
    pub stderr: String,
}
pub fn apply_patch_text(patch: &str) -> ApplyResult {
    match apply_patch_text_inner(patch) {
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
fn apply_patch_text_inner(patch: &str) -> anyhow::Result<Summary> {
    let hunks = parse_patch(patch)?;
    anyhow::ensure!(!hunks.is_empty(), "No files were modified.");
    let mut summary = Summary::default();
    for hunk in hunks {
        if let Err(error) = apply_hunk(hunk, &mut summary) {
            summary.errors.push(error.to_string());
        }
    }
    Ok(summary)
}
fn apply_hunk(hunk: FileHunk, summary: &mut Summary) -> anyhow::Result<()> {
    match hunk {
        FileHunk::Add {
            path,
            contents,
            line_count,
            character_count,
        } => {
            let after = FileStats::from_counts(line_count, character_count);
            FileWriter::write_with_parent_retry(&path, contents)?;
            summary
                .added
                .push(FileChange::new(path, FileStats::empty(), after));
        }
        FileHunk::Delete { path } => {
            let (target, original_contents) = FileWriter::read_file_to_delete(&path)?;
            let before = FileStats::from_contents(&original_contents);
            FileWriter::delete_resolved_file(&target)?;
            summary
                .deleted
                .push(FileChange::new(path, before, FileStats::empty()));
        }
        FileHunk::Update {
            path,
            move_path,
            chunks,
        } => {
            let (source, original_contents) = FileWriter::read_file_to_update(&path)?;
            if chunks.is_empty() {
                let before = FileStats::from_contents(&original_contents);
                if let Some(destination_path) = move_path {
                    FileWriter::write_with_parent_retry(&destination_path, original_contents)?;
                    FileWriter::delete_resolved_original(&source)?;
                    summary
                        .modified
                        .push(FileChange::new(destination_path, before, before));
                }
                return Ok(());
            }
            let derived = derive_new_contents(&source, &original_contents, &chunks);
            let before = derived.before;
            summary.errors.extend(derived.errors);
            if let Some(destination_path) = move_path {
                if derived.applied_chunks > 0 {
                    let after = FileStats::from_contents(&derived.contents);
                    FileWriter::write_with_parent_retry(&destination_path, derived.contents)?;
                    FileWriter::delete_resolved_original(&source)?;
                    summary
                        .modified
                        .push(FileChange::new(destination_path, before, after));
                }
            } else if derived.applied_chunks > 0 {
                let after = FileStats::from_contents(&derived.contents);
                FileWriter::write_resolved_file(&source, derived.contents)?;
                summary.modified.push(FileChange::new(path, before, after));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::apply_patch_text;
    use std::fs;
    #[test]
    fn failed_chunk_does_not_stop_following_chunks_in_same_file() {
        let directory = tempfile::tempdir().unwrap();
        let target_path = directory.path().join("target.txt");
        fs::write(&target_path, "one\ntwo\nthree\n").unwrap();
        let patch = [
            "*** Begin Patch",
            &format!("*** Update File: {}", target_path.display()),
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
        let result = apply_patch_text(&patch);
        assert!(
            result
                .stdout
                .contains(&format!("M {}", target_path.display()))
        );
        assert!(result.stderr.contains("Failed to find expected lines"));
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "1\ntwo\n3\n");
    }
    #[test]
    fn output_includes_file_stats_before_and_after_changes() {
        let directory = tempfile::tempdir().unwrap();
        let target_path = directory.path().join("target.txt");
        let obsolete_path = directory.path().join("obsolete.txt");
        fs::write(&target_path, "old\n").unwrap();
        fs::write(&obsolete_path, "bye\n").unwrap();
        let patch = [
            "*** Begin Patch",
            &format!(
                "*** Add File: {}",
                directory.path().join("hello.txt").display()
            ),
            "+hello",
            "+world",
            &format!("*** Update File: {}", target_path.display()),
            "@@",
            "-old",
            "+new",
            &format!("*** Delete File: {}", obsolete_path.display()),
            "*** End Patch",
            "",
        ]
        .join("\n");
        let result = apply_patch_text(&patch);
        assert!(result.stderr.is_empty(), "{}", result.stderr);
        assert!(result.stdout.contains(&format!(
            "A {} (before: 0 lines, 0 chars; after: 2 lines, 12 chars)",
            directory.path().join("hello.txt").display()
        )));
        assert!(result.stdout.contains(&format!(
            "M {} (before: 1 lines, 4 chars; after: 1 lines, 4 chars)",
            target_path.display()
        )));
        assert!(result.stdout.contains(&format!(
            "D {} (before: 1 lines, 4 chars; after: 0 lines, 0 chars)",
            obsolete_path.display()
        )));
    }
}
