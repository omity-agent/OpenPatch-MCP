use super::{derive_new_contents, replacements::Replacement, replacements::apply_replacements};
use crate::parser::UpdateChunk;
#[test]
fn insertion_without_old_lines_precedes_logical_trailing_empty_line() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: Vec::new().into(),
        new_lines: vec![String::from("b")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("a\n\n", &[chunk]);
    assert_eq!(result.contents, "a\nb\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn replacement_can_ignore_empty_lines_in_original() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from("a"), String::from("b")].into(),
        new_lines: vec![String::from("updated")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("a\n\nb\nc\n", &[chunk]);
    assert_eq!(result.contents, "updated\nc\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn replacement_can_ignore_consecutive_space_counts() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from("a b"), String::from("c d")].into(),
        new_lines: vec![String::from("updated")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("a  b\n\nc   d\n", &[chunk]);
    assert_eq!(result.contents, "updated\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn replacements_are_applied_in_one_forward_pass() {
    let owned_lines = (0_usize..1_000_usize)
        .map(|index| format!("line-{index}"))
        .collect::<Vec<_>>();
    let original_lines = owned_lines.iter().map(String::as_str).collect::<Vec<_>>();
    let original_contents = owned_lines.join("\n");
    let offsets = line_offsets(&original_lines);
    let replacements = (0_usize..1_000_usize)
        .step_by(2)
        .map(|index| (index, 1_usize, vec![format!("updated-{index}")]))
        .collect::<Vec<_>>();
    let borrowed_replacements = replacements
        .iter()
        .map(|replacement| (replacement.0, replacement.1, replacement.2.as_slice()))
        .collect::<Vec<Replacement>>();
    let result = apply_replacements(
        &original_contents,
        &original_lines,
        &offsets,
        &borrowed_replacements,
    );
    let result_lines = result
        .trim_end_matches('\n')
        .split('\n')
        .collect::<Vec<_>>();
    assert_eq!(result_lines.len(), 1_000);
    assert_eq!(result_lines.first().copied(), Some("updated-0"));
    assert_eq!(result_lines.get(1).copied(), Some("line-1"));
    assert_eq!(result_lines.get(998).copied(), Some("updated-998"));
    assert_eq!(result_lines.get(999).copied(), Some("line-999"));
}
#[test]
fn adjacent_insertions_keep_patch_order() {
    let original_contents = "a";
    let original_lines = ["a"];
    let offsets = line_offsets(&original_lines);
    let first = [String::from("b")];
    let second = [String::from("c")];
    let replacements = vec![(1, 0, first.as_slice()), (1, 0, second.as_slice())];
    let result = apply_replacements(original_contents, &original_lines, &offsets, &replacements);
    assert_eq!(result, "a\nb\nc\n");
}
fn line_offsets(lines: &[&str]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(lines.len());
    let mut offset = 0;
    for line in lines {
        offsets.push(offset);
        offset += line.len() + 1;
    }
    offsets
}
