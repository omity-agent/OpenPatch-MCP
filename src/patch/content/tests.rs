use super::{derive_new_contents, replacements::Replacement, replacements::apply_replacements};
use crate::parser::UpdateChunk;
use std::path::Path;
#[test]
fn insertion_without_old_lines_precedes_logical_trailing_empty_line() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: Vec::new(),
        new_lines: vec![String::from("b")],
        is_end_of_file: false,
    };
    let result = derive_new_contents(Path::new("target.txt"), "a\n\n", &[chunk]);
    assert_eq!(result.contents, "a\nb\n");
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
    let replacements = (0_usize..1_000_usize)
        .step_by(2)
        .map(|index| (index, 1_usize, vec![format!("updated-{index}")]))
        .collect::<Vec<Replacement>>();
    let result = apply_replacements(&original_contents, &original_lines, &replacements);
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
    let replacements = vec![
        (1, 0, vec![String::from("b")]),
        (1, 0, vec![String::from("c")]),
    ];
    let result = apply_replacements(original_contents, &original_lines, &replacements);
    assert_eq!(result, "a\nb\nc\n");
}
