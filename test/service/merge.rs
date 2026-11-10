use super::reverse_contents;
#[test]
fn preserves_unrelated_edits_and_rejects_conflicts() {
    for (before, after, local, expected) in [
        (
            "old\nkeep\n",
            "new\nkeep\n",
            "new\nchanged\n",
            Some("old\nchanged\n"),
        ),
        (
            "head\nold\n",
            "head\nnew\n",
            "local\nhead\nnew\n",
            Some("local\nhead\nold\n"),
        ),
        ("old\n", "new\n", "modified\n", None),
        (
            "old\nkeep\nnew\n",
            "new\nkeep\nnew\n",
            "modified\nkeep\nnew\n",
            None,
        ),
        ("old\nkeep\n", "keep\n", "local\nkeep\n", None),
    ] {
        let result = reverse_contents(before, after, local).map_err(|error| error.to_string());
        assert_eq!(
            result.as_deref().map_err(String::as_str),
            expected.ok_or(super::CONFLICT_REASON)
        );
    }
}
