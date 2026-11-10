use super::{apply, assert_contents, id_from, target, undo};
#[test]
fn cumulative_rules_match_without_rewriting_unmatched_text() {
    for (original, pattern) in [
        ("\u{FEFF}  target\u{200B}  ", "target"),
        ("ta\u{200C}\u{200D}\u{2060}\u{202E}rget", "target"),
        ("e\u{034F}\u{0301}", "é"),
        ("⭐\u{FE0F}", "⭐"),
        ("a\tb", "a b"),
        ("a\u{1680}b", "a b"),
        ("Ｆｕｌｌ ﬁ", "Full fi"),
        ("a—“b”", "a-\"b\""),
        ("раураl", "paypal"),
        ("ΑΒΕΙΚΜΝΟΡΤΥΧ", "ABEIKMNOPTYX"),
        ("α", "а"),
        ("β", "ß"),
        ("ё", "ë"),
        ("ό", "ó"),
        (
            "\u{FEFF}  ｐа—“b”\t  c  \n\u{200B}\nlast",
            "pa-\"b\" c\nlast",
        ),
    ] {
        for newline in ["\n", "\r\n"] {
            let contents =
                format!("untouched\tα\n{original}\ntail\u{200B}\n").replace('\n', newline);
            let (_directory, service, path) = target(&contents);
            let applied = service.replace(&path.display().to_string(), pattern, "changed");
            assert!(applied.succeeded(), "{original:?}: {}", applied.render());
            assert_contents(
                &path,
                Some(&format!(
                    "untouched\tα{newline}changed{newline}tail\u{200B}{newline}"
                )),
            );
            undo(&service, &id_from(&applied.render()));
            assert_contents(&path, Some(&contents));
        }
    }
}
#[test]
fn rules_do_not_introduce_unrelated_equivalences() {
    for (original, pattern) in [
        ("A", "a"),
        ("O", "0"),
        ("I", "l"),
        ("m", "rn"),
        ("ı", "i"),
        ("a\tb", "ab"),
        ("é", "e"),
        ("a\0b", "ab"),
        ("a", "\u{200B}"),
    ] {
        let (_directory, service, path) = target(original);
        let applied = service.replace(&path.display().to_string(), pattern, "changed");
        assert!(!applied.succeeded(), "{original:?} matched {pattern:?}");
        assert_contents(&path, Some(original));
    }
}
#[test]
fn lower_risk_matches_win_over_earlier_higher_risk_matches() {
    for (original, pattern, expected) in [
        (" target\ntarget\n", "target", " target\nchanged\n"),
        (
            "t\u{200B}arget\n target \n",
            "target",
            "t\u{200B}arget\nchanged\n",
        ),
        ("a\u{1680}b\na\u{200B} b\n", "a b", "a\u{1680}b\nchanged\n"),
        ("a\tb\na\u{1680}b\n", "a b", "a\tb\nchanged\n"),
        ("ａ b\na\tb\n", "a b", "ａ b\nchanged\n"),
        ("Ａ\nA\t\n", "A", "Ａ\nchanged\n"),
        ("a—b\nａ-b\n", "a-b", "a—b\nchanged\n"),
        ("a—b\nc\na-b\n\nc\n", "a-b\nc", "changed\na-b\n\nc\n"),
        ("a  b\nc\na b\n\nc\n", "a b\nc", "a  b\nc\nchanged\n"),
        ("р q\np\tq\n", "p q", "р q\nchanged\n"),
        ("р q\np  q\n", "p q", "р q\nchanged\n"),
    ] {
        let (_directory, service, path) = target(original);
        let applied = service.replace(&path.display().to_string(), pattern, "changed");
        assert!(applied.succeeded(), "{}", applied.render());
        assert_contents(&path, Some(expected));
    }
}
#[test]
fn context_and_eof_share_the_same_rule_chain() {
    let original = "heading\n\u{FEFF}  functiοn\talpha  \nold\nold\n";
    let (_directory, service, path) = target(original);
    let applied = apply(
        &service,
        &format!(
            "*** Update File: {}\n@@ function alpha\n-old\n+changed\n*** End of File",
            path.display()
        ),
    );
    assert_contents(
        &path,
        Some("heading\n\u{FEFF}  functiοn\talpha  \nold\nchanged\n"),
    );
    undo(&service, &id_from(&applied));
    assert_contents(&path, Some(original));
    let overlapping = service.apply(&format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-old\n+changed\n*** End of File\n@@\n-old\n+again\n*** End of File\n*** End Patch",
        path.display()
    ));
    assert!(!overlapping.succeeded(), "{}", overlapping.render());
    assert_contents(
        &path,
        Some("heading\n\u{FEFF}  functiοn\talpha  \nold\nchanged\n"),
    );
}
#[test]
fn diagnostics_normalize_for_alignment_but_report_original_text() {
    let original = "\u{200B}\n\u{FEFF}рау\tload — “alpha”\nunrelated\n";
    let (_directory, service, path) = target(original);
    let applied = service.replace(
        &path.display().to_string(),
        "pay load - \"alphx\"",
        "changed",
    );
    assert!(!applied.succeeded());
    assert!(applied.render().contains("\u{FEFF}рау\tload — “alpha”"));
    assert_contents(&path, Some(original));
}
