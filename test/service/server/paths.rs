use super::{Application, InputStyle, call, connect, json, output_text, output_value};
use core::fmt::Write as _;
use std::fs;
#[tokio::test]
async fn apply_and_undo_share_component_based_path_labels() {
    for (names, labels) in [
        (vec!["nested/one.txt"], vec!["one.txt"]),
        (vec!["src/one.rs", "src/two.rs"], vec!["one.rs", "two.rs"]),
        (
            vec!["src/a.rs", "tests/b.rs"],
            vec!["src/a.rs", "tests/b.rs"],
        ),
        (
            vec!["src/a.rs", "src-extra/a.rs"],
            vec!["src/a.rs", "src-extra/a.rs"],
        ),
        (
            vec!["源码/a name.rs", "用例/a name.rs"],
            vec!["源码/a name.rs", "用例/a name.rs"],
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let application =
            Application::with_database(&directory.path().join("paths.lmdb"), InputStyle::Openai)
                .unwrap();
        let (client, server) = connect(application).await;
        let mut patch = String::from("*** Begin Patch\r\n");
        for name in &names {
            writeln!(
                patch,
                "*** Add File: {}\r\n+contents",
                directory.path().join(name).display()
            )
            .unwrap();
        }
        patch.push_str("*** End Patch");
        let applied = call(&client, "apply_patch", json!({"patch": patch})).await;
        assert_eq!(applied.is_error, Some(false));
        let successes = output_value(&applied, "/successes").as_array().unwrap();
        for (success, label) in successes.iter().zip(&labels) {
            assert_eq!(success["path"], *label);
            assert!(output_text(&applied).contains(&format!("<ADD>\n{label}\n")));
        }
        for name in names {
            assert_eq!(
                fs::read_to_string(directory.path().join(name)).unwrap(),
                "contents\n"
            );
        }
        let ids = successes
            .iter()
            .map(|success| success["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        let undone = call(&client, "undo_patch", json!({"ids": ids})).await;
        assert_eq!(undone.is_error, Some(false));
        for (success, label) in output_value(&undone, "/successes")
            .as_array()
            .unwrap()
            .iter()
            .zip(labels.iter().rev())
        {
            assert_eq!(success["path"], *label);
            assert!(output_text(&undone).contains(&format!("<DELETE>\n{label}\n")));
        }
        for result in [&applied, &undone] {
            assert!(!output_text(result).contains(&directory.path().display().to_string()));
        }
        client.cancel().await.unwrap();
        server.await.unwrap().unwrap();
    }
}
#[tokio::test]
async fn mixed_failures_and_expansion_errors_do_not_expose_absolute_paths() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let blocked = root.join("tests/a.rs");
    fs::create_dir_all(&blocked).unwrap();
    let application =
        Application::with_database(&root.join("failures.lmdb"), InputStyle::Openai).unwrap();
    let (client, server) = connect(application).await;
    let patch = format!(
        "*** Begin Patch\n*** Add File: {}\n+contents\n*** Update File: {}\n@@\n-old\n+new\n*** End Patch",
        root.join("src/a.rs").display(),
        blocked.display()
    );
    let failed = call(&client, "apply_patch", json!({"patch": patch})).await;
    assert_eq!(failed.is_error, Some(true));
    assert_eq!(*output_value(&failed, "/successes/0/path"), "src/a.rs");
    assert_eq!(
        *output_value(&failed, "/failures/0/operation/path"),
        "tests/a.rs"
    );
    assert!(
        output_value(&failed, "/failures/0/reason")
            .as_str()
            .unwrap()
            .contains("tests/a.rs")
    );
    let invalid = root.join("${}.txt");
    let malformed = call(&client, "apply_patch", json!({"patch": format!("*** Begin Patch\n*** Add File: {}\n+contents\n*** End Patch", invalid.display())})).await;
    assert_eq!(malformed.is_error, Some(true));
    assert!(
        output_value(&malformed, "/failures/0/reason")
            .as_str()
            .unwrap()
            .contains("'${}.txt'")
    );
    for result in [&failed, &malformed] {
        let reason = output_value(result, "/failures/0/reason").as_str().unwrap();
        assert!(!reason.contains(&root.display().to_string()), "{reason}");
        assert!(!output_text(result).contains(&root.display().to_string()));
    }
    client.cancel().await.unwrap();
    server.await.unwrap().unwrap();
}
