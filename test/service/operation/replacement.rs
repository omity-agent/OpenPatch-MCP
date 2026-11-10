use super::{apply, id_from, target, undo};
use std::fs;
#[test]
fn matching_replacements_can_be_undone() {
    for (original, old, new, expected) in [
        (
            "alpha  beta\n\ncharlie   delta\n",
            "alpha beta\ncharlie delta",
            "changed",
            "changed\n",
        ),
        ("same\nsame\n", "same", "changed", "changed\nsame\n"),
        ("old value\n", "old value", "new value", "new value\n"),
    ] {
        let (_directory, service, target) = target(original);
        let output = service.replace(&target.display().to_string(), old, new);
        assert!(output.succeeded(), "{}", output.render());
        assert_eq!(fs::read_to_string(&target).unwrap(), expected);
        undo(&service, &id_from(&output.render()));
        assert_eq!(fs::read_to_string(target).unwrap(), original);
    }
}
#[test]
fn multiple_chunks_keep_original_line_positions_when_blank_lines_are_skipped() {
    for newline in ["\n", "\r\n"] {
        let prefix = newline.repeat(32);
        let original = format!(
            "{prefix}定位{newline}alpha  beta{newline}{newline}charlie   delta{newline}尾部{newline}"
        );
        let (_directory, service, target) = target(&original);
        let applied = apply(
            &service,
            &format!(
                "*** Update File: {}\n@@\n-定位\n+新的定位\n@@\n-alpha beta\n-charlie delta\n+替换内容",
                target.display()
            ),
        );
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            format!("{prefix}新的定位{newline}替换内容{newline}尾部{newline}")
        );
        undo(&service, &id_from(&applied));
        assert_eq!(fs::read_to_string(target).unwrap(), original);
    }
}
#[test]
fn repeated_failed_chunks_report_unicode_context_without_changing_the_file() {
    let original = "\n\n你好世界，第一段内容\n\n另一个段落，完整内容\n\n";
    let (_directory, service, target) = target(original);
    let applied = service.apply(&format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-你好世界，第一段内客\n+replacement\n@@\n-另一个段落，完整内客\n+replacement\n*** End Patch",
        target.display()
    ));
    assert!(!applied.succeeded());
    let rendered = applied.render();
    assert_eq!(rendered.matches("Closest match:").count(), 2, "{rendered}");
    assert!(rendered.contains("你好世界，第一段内容"), "{rendered}");
    assert!(rendered.contains("另一个段落，完整内容"), "{rendered}");
    assert_eq!(fs::read_to_string(target).unwrap(), original);
}
