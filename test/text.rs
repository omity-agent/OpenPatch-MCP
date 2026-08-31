use super::LineEndings;
#[test]
fn preserves_a_consistent_line_ending_sequence() {
    for (contents, expected) in [
        ("first\nsecond\n", "changed\nnext\n"),
        ("first\r\nsecond\r\n", "changed\r\nnext\r\n"),
        ("first\rsecond\r", "changed\rnext\r"),
    ] {
        let endings = LineEndings::detect(contents);
        assert!(!endings.is_mixed());
        assert_eq!(endings.render(String::from("changed\nnext\n")), expected);
    }
}
#[test]
fn normalizes_mixed_sequences_to_lf() {
    let contents = "first\r\nsecond\nthird\rfourth";
    let endings = LineEndings::detect(contents);
    assert!(endings.is_mixed());
    assert_eq!(
        LineEndings::normalize(contents),
        "first\nsecond\nthird\nfourth"
    );
    assert_eq!(
        endings.render(String::from("changed\nnext\n")),
        "changed\nnext\n"
    );
}
