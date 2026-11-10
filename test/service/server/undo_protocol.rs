use super::{CallToolResult, Client, InputStyle, call, connect, json, output_text, output_value};
use crate::operation::model::OperationId;
use std::fs;
#[tokio::test]
async fn undo_accepts_ids_and_reports_short_ids_in_both_output_formats() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("undo-target.txt");
    fs::write(&target, "before\n").unwrap();
    let (client, server) = connect(&directory, InputStyle::Openai).await;
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-before\n+after\n*** End Patch",
        target.display()
    );
    let applied = call(&client, "apply_patch", json!({"patch": patch})).await;
    assert_eq!(applied.is_error, Some(false));
    let original = output_id(&applied);
    let undo = call_undo(&client, &[original, "not-an-id"]).await;
    assert_eq!(undo.is_error, Some(true));
    assert_eq!(*output_value(&undo, "/failures/0/undoId"), "not-an-id");
    let reversal = output_id(&undo);
    assert_ne!(reversal, original);
    assert_eq!(*output_value(&undo, "/successes/0/undoOf"), original);
    let text = output_text(&undo);
    assert!(text.contains(&format!("<ID>\n{reversal}\n</ID>")));
    assert!(text.contains(&format!("<UNDO_OF>\n{original}\n</UNDO_OF>")));
    assert_eq!(fs::read_to_string(&target).unwrap(), "before\n");
    let duplicate = call_undo(&client, &[original]).await;
    assert_eq!(duplicate.is_error, Some(true));
    let failure = output_value(&duplicate, "/failures/0");
    assert_eq!(failure["undoId"], original);
    assert!(failure["reason"].as_str().unwrap().contains(reversal));
    let redone = call_undo(&client, &[reversal]).await;
    assert_eq!(redone.is_error, Some(false));
    assert_eq!(fs::read_to_string(&target).unwrap(), "after\n");
    let empty = call_undo(&client, &[]).await;
    assert_eq!(empty.is_error, Some(true));
    assert_eq!(
        *output_value(&empty, "/failures/0/reason"),
        "ids must not be empty"
    );
    client.cancel().await.unwrap();
    server.await.unwrap().unwrap();
}
fn output_id(result: &CallToolResult) -> &str {
    let id = output_value(result, "/successes/0/id").as_str().unwrap();
    assert_eq!(OperationId::parse(id).unwrap().to_string(), id);
    id
}
async fn call_undo(client: &Client, ids: &[&str]) -> CallToolResult {
    call(client, "undo_patch", json!({"ids": ids})).await
}
