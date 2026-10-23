use super::{id_from, service};
use std::fs;
#[test]
fn uses_fuzzy_matching() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "alpha  beta\n\ncharlie   delta\n").unwrap();
    let path = target.display().to_string();
    let output = service.replace(&path, "alpha beta\ncharlie delta", "changed");
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "changed\n");
}
#[test]
fn replaces_the_first_exact_match() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "same\nsame\n").unwrap();
    let path = target.display().to_string();
    let output = service.replace(&path, "same", "changed");
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "changed\nsame\n");
}
#[test]
fn can_be_undone() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("target.txt");
    fs::write(&target, "old value\n").unwrap();
    let path = target.display().to_string();
    let output = service.replace(&path, "old value", "new value");
    assert!(output.succeeded(), "{}", output.render());
    assert_eq!(fs::read_to_string(&target).unwrap(), "new value\n");
    let undone = service.undo(&[id_from(&output.render())]);
    assert!(undone.succeeded(), "{}", undone.render());
    assert_eq!(fs::read_to_string(target).unwrap(), "old value\n");
}
#[test]
fn multiple_chunks_keep_original_line_positions_when_blank_lines_are_skipped() {
    for newline in ["\n", "\r\n"] {
        let directory = tempfile::tempdir().unwrap();
        let service = service(&directory);
        let target = directory.path().join("unicode.txt");
        let prefix = newline.repeat(32);
        let original = format!(
            "{prefix}定位{newline}alpha  beta{newline}{newline}charlie   delta{newline}尾部{newline}"
        );
        fs::write(&target, &original).unwrap();
        let applied = service.apply(&format!(
            "*** Begin Patch\n*** Update File: {}\n@@\n-定位\n+新的定位\n@@\n-alpha beta\n-charlie delta\n+替换内容\n*** End Patch",
            target.display()
        ));
        assert!(applied.succeeded(), "{}", applied.render());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            format!("{prefix}新的定位{newline}替换内容{newline}尾部{newline}")
        );
        let undone = service.undo(&[id_from(&applied.render())]);
        assert!(undone.succeeded(), "{}", undone.render());
        assert_eq!(fs::read_to_string(target).unwrap(), original);
    }
}
#[test]
fn repeated_failed_chunks_report_unicode_context_without_changing_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let service = service(&directory);
    let target = directory.path().join("diagnostics.txt");
    let original = "\n\n你好世界，第一段内容\n\n另一个段落，完整内容\n\n";
    fs::write(&target, original).unwrap();
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
