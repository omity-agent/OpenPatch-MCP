use super::Application;
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientRequest, Request},
};
use std::fs;
#[derive(Clone, Default)]
struct TestClient;
#[expect(
    clippy::missing_trait_methods,
    reason = "default client handlers are enough for this in-process test"
)]
impl ClientHandler for TestClient {}
#[tokio::test]
async fn mcp_call_applies_multiline_patch() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "old\n").unwrap();
    let application =
        Application::with_database(&directory.path().join("history.sqlite3")).unwrap();
    verify_schemas(&application);
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let server_handle = tokio::spawn(async move {
        let service = ServiceExt::serve(application, server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });
    let client = ServiceExt::serve(TestClient, client_transport)
        .await
        .unwrap();
    let patch = [
        "*** Begin Patch",
        &format!("*** Update File: {}", target_path.display()),
        "@@",
        "-old",
        "+new",
        "*** End Patch",
        "",
    ]
    .join("\n");
    let tool_result = call_apply_patch(&client, &patch).await;
    assert_eq!(tool_result.is_error, Some(false));
    let content = tool_result
        .content
        .first()
        .unwrap()
        .as_text()
        .unwrap()
        .text
        .as_str();
    assert!(content.find("after:").unwrap() < content.find("<UUID>").unwrap());
    verify_success(&tool_result.structured_content.unwrap(), &target_path);
    let failed_result = call_apply_patch(&client, "").await;
    assert_eq!(failed_result.is_error, Some(true));
    verify_failure(&failed_result.structured_content.unwrap());
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "new\n");
    client.cancel().await.unwrap();
    server_handle.await.unwrap().unwrap();
}
fn verify_schemas(application: &Application) {
    let tools = application.tool_router.list_all();
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|tool| tool.output_schema.is_some()));
    for tool in &tools {
        let schema = rmcp::serde_json::to_string(tool.output_schema.as_ref().unwrap()).unwrap();
        assert!(!schema.contains(r#""format":"uint""#));
    }
}
async fn call_apply_patch(
    client: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    patch: &str,
) -> rmcp::model::CallToolResult {
    let arguments = rmcp::model::object(rmcp :: serde_json :: json ! ({ "patch" : patch }));
    let request = ClientRequest::CallToolRequest(Request::new(
        CallToolRequestParams::new("apply_patch").with_arguments(arguments),
    ));
    let result = client.peer().send_request(request).await.unwrap();
    let rmcp::model::ServerResult::CallToolResult(tool_result) = result else {
        panic!("expected call tool result");
    };
    tool_result
}
fn verify_success(structured: &rmcp::serde_json::Value, target_path: &std::path::Path) {
    let result = structured.as_object().unwrap();
    assert_eq!(
        result.get("succeeded"),
        Some(&rmcp::serde_json::json!(true))
    );
    let success = result
        .get("successes")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        success.keys().map(String::as_str).collect::<Vec<_>>(),
        ["kind", "path", "before", "after", "uuid", "undoOf"]
    );
    assert_eq!(success.get("kind"), Some(&rmcp::serde_json::json!("EDIT")));
    assert_eq!(
        success.get("path").unwrap().as_str().unwrap(),
        target_path.display().to_string()
    );
    assert_eq!(
        success
            .get("before")
            .unwrap()
            .get("lineCount")
            .unwrap()
            .as_u64(),
        Some(1)
    );
    assert_eq!(
        success
            .get("after")
            .unwrap()
            .get("lineCount")
            .unwrap()
            .as_u64(),
        Some(1)
    );
    assert!(success.get("uuid").unwrap().is_string());
    assert!(success.get("undoOf").unwrap().is_null());
    assert!(
        result
            .get("failures")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
}
fn verify_failure(structured: &rmcp::serde_json::Value) {
    let result = structured.as_object().unwrap();
    assert_eq!(
        result.get("succeeded"),
        Some(&rmcp::serde_json::json!(false))
    );
    assert!(
        result
            .get("successes")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    let failure = result
        .get("failures")
        .unwrap()
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_object()
        .unwrap();
    assert!(failure.get("operation").unwrap().is_null());
    assert!(failure.get("undoUuid").unwrap().is_null());
    assert_eq!(
        failure.get("reason"),
        Some(&rmcp::serde_json::json!("patch must not be empty"))
    );
}
