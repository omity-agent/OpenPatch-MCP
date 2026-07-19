use super::super::derive_new_contents;
use crate::parser::UpdateChunk;
#[test]
fn already_applied_replacement_reconstructs_patch_before_contents() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from("old from patch")].into(),
        new_lines: vec![String::from("new")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("new\n", &[chunk]);
    assert_eq!(result.contents, "new\n");
    assert_eq!(result.before_contents, "old from patch\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn target_contents_must_match_exactly() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from("old")].into(),
        new_lines: vec![String::from("new")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents(" new \n", &[chunk]);
    assert_eq!(result.applied_chunks, 0);
    assert_eq!(result.errors.len(), 1);
}
#[test]
fn exact_target_match_precedes_a_fuzzy_old_match() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from(" new ")].into(),
        new_lines: vec![String::from("new")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("new\n", &[chunk]);
    assert_eq!(result.contents, "new\n");
    assert_eq!(result.before_contents, " new \n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn already_applied_eof_deletion_restores_old_lines_at_the_end() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: vec![String::from("removed")].into(),
        new_lines: Vec::new().into(),
        is_end_of_file: true,
    };
    let result = derive_new_contents("kept\n", &[chunk]);
    assert_eq!(result.contents, "kept\n");
    assert_eq!(result.before_contents, "kept\nremoved\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
#[test]
fn mixed_update_reconstructs_full_before_and_after_contents() {
    let chunks = [
        UpdateChunk {
            change_context: None,
            old_lines: vec![String::from("old one")].into(),
            new_lines: vec![String::from("new one")].into(),
            is_end_of_file: false,
        },
        UpdateChunk {
            change_context: None,
            old_lines: vec![String::from("old two")].into(),
            new_lines: vec![String::from("new two")].into(),
            is_end_of_file: false,
        },
    ];
    let result = derive_new_contents("new one\nold two\n", &chunks);
    assert_eq!(result.before_contents, "old one\nold two\n");
    assert_eq!(result.contents, "new one\nnew two\n");
    assert_eq!(result.applied_chunks, 2);
    assert!(result.errors.is_empty());
}
#[test]
fn multiple_already_applied_insertions_are_reconstructed_in_patch_order() {
    let chunks = [
        UpdateChunk {
            change_context: None,
            old_lines: Vec::new().into(),
            new_lines: vec![String::from("first")].into(),
            is_end_of_file: false,
        },
        UpdateChunk {
            change_context: None,
            old_lines: Vec::new().into(),
            new_lines: vec![String::from("second")].into(),
            is_end_of_file: false,
        },
    ];
    let result = derive_new_contents("kept\nfirst\nsecond\n", &chunks);
    assert_eq!(result.before_contents, "kept\n");
    assert_eq!(result.contents, "kept\nfirst\nsecond\n");
    assert_eq!(result.applied_chunks, 2);
    assert!(result.errors.is_empty());
}
#[test]
fn insertion_target_in_the_middle_is_not_already_applied() {
    let chunk = UpdateChunk {
        change_context: None,
        old_lines: Vec::new().into(),
        new_lines: vec![String::from("added")].into(),
        is_end_of_file: false,
    };
    let result = derive_new_contents("added\nkept\n", &[chunk]);
    assert_eq!(result.before_contents, "added\nkept\n");
    assert_eq!(result.contents, "added\nkept\nadded\n");
    assert_eq!(result.applied_chunks, 1);
    assert!(result.errors.is_empty());
}
