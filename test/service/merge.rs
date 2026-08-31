use super::reverse_contents;
#[test]
fn preserves_an_adjacent_unrelated_edit() {
    let result = reverse_contents("old\nkeep\n", "new\nkeep\n", "new\nchanged\n").unwrap();
    assert_eq!(result, "old\nchanged\n");
}
#[test]
fn preserves_an_unrelated_leading_insertion() {
    let result = reverse_contents("head\nold\n", "head\nnew\n", "local\nhead\nnew\n").unwrap();
    assert_eq!(result, "local\nhead\nold\n");
}
#[test]
fn rejects_an_overlapping_edit() {
    let error = reverse_contents("old\n", "new\n", "modified\n").unwrap_err();
    assert_eq!(error.to_string(), super::CONFLICT_REASON);
}
#[test]
fn rejects_an_ambiguous_match_after_an_overlapping_edit() {
    let error = reverse_contents(
        "old\nkeep\nnew\n",
        "new\nkeep\nnew\n",
        "modified\nkeep\nnew\n",
    )
    .unwrap_err();
    assert_eq!(error.to_string(), super::CONFLICT_REASON);
}
#[test]
fn rejects_insertions_at_the_same_position() {
    let error = reverse_contents("old\nkeep\n", "keep\n", "local\nkeep\n").unwrap_err();
    assert_eq!(error.to_string(), super::CONFLICT_REASON);
}
