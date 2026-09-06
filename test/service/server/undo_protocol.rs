use super::{Application, InputStyle, TestClient, call_apply_patch};
use crate::operation::model::OperationId;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientRequest, Request},
};
use std::fs;
#[tokio::test]
async fn undo_accepts_ids_and_reports_short_ids_in_both_output_formats() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("undo-target.txt");
    fs::write(&target, "before\n").unwrap();
    let application = Application::with_database(
        &directory.path().join("protocol.sqlite3"),
        InputStyle::Openai,
    )
    .unwrap();
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let server_handle = tokio::spawn(async move {
        let service = ServiceExt::serve(application, server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });
    let client = ServiceExt::serve(TestClient, client_transport)
        .await
        .unwrap();
    let patch = format!(
        "*** Begin Patch\n*** Update File: {}\n@@\n-before\n+after\n*** End Patch",
        target.display()
    );
    let applied = call_apply_patch(&client, &patch).await;
    assert_eq!(applied.is_error, Some(false));
    let original = output_id(&applied).to_owned();
    let undo = call_undo(&client, &[original.as_str()]).await;
    assert_eq!(undo.is_error, Some(false));
    let reversal_id = output_id(&undo);
    assert_ne!(reversal_id, original);
    let structured = undo.structured_content.as_ref().unwrap();
    assert_eq!(
        structured.pointer("/successes/0/undoOf").unwrap().as_str(),
        Some(original.as_str())
    );
    let text = undo
        .content
        .first()
        .unwrap()
        .as_text()
        .unwrap()
        .text
        .as_str();
    assert!(text.contains(&format!("<ID>\n{reversal_id}\n</ID>")));
    assert!(text.contains(&format!("<UNDO_OF>\n{original}\n</UNDO_OF>")));
    assert_eq!(fs::read_to_string(&target).unwrap(), "before\n");
    let duplicate = call_undo(&client, &[original.as_str()]).await;
    assert_eq!(duplicate.is_error, Some(true));
    let failure = duplicate.structured_content.as_ref().unwrap();
    assert_eq!(
        failure.pointer("/failures/0/undoId").unwrap().as_str(),
        Some(original.as_str())
    );
    assert!(
        failure
            .pointer("/failures/0/reason")
            .unwrap()
            .as_str()
            .unwrap()
            .contains(reversal_id)
    );
    let redone = call_undo(&client, &[reversal_id]).await;
    assert_eq!(redone.is_error, Some(false));
    assert_eq!(fs::read_to_string(&target).unwrap(), "after\n");
    let invalid = call_undo(&client, &["oppc-ABC12345"]).await;
    assert_eq!(invalid.is_error, Some(true));
    let rejected = invalid.structured_content.as_ref().unwrap();
    assert_eq!(
        rejected.pointer("/failures/0/undoId").unwrap().as_str(),
        Some("oppc-ABC12345")
    );
    let empty = call_undo(&client, &[]).await;
    assert_eq!(empty.is_error, Some(true));
    assert_eq!(
        empty
            .structured_content
            .as_ref()
            .unwrap()
            .pointer("/failures/0/reason")
            .unwrap()
            .as_str(),
        Some("ids must not be empty")
    );
    client.cancel().await.unwrap();
    server_handle.await.unwrap().unwrap();
}
fn output_id(result: &rmcp::model::CallToolResult) -> &str {
    let id = result
        .structured_content
        .as_ref()
        .unwrap()
        .pointer("/successes/0/id")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(OperationId::parse(id).unwrap().to_string(), id);
    id
}
async fn call_undo(
    client: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    ids: &[&str],
) -> rmcp::model::CallToolResult {
    let mut arguments = rmcp::serde_json::Map::new();
    arguments.insert(String::from("ids"), rmcp::serde_json::json!(ids));
    let request = ClientRequest::CallToolRequest(Request::new(
        CallToolRequestParams::new("undo_patch").with_arguments(arguments),
    ));
    let result = client.peer().send_request(request).await.unwrap();
    let rmcp::model::ServerResult::CallToolResult(tool_result) = result else {
        panic!("expected call tool result");
    };
    tool_result
}
