use super::Application;
use crate::service::cli::InputStyle;
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientRequest, Request},
};
use std::fs;
mod general;
mod structured_result;
use structured_result::{verify_failure, verify_success};
#[derive(Clone, Default)]
struct TestClient;
#[expect(
    clippy::missing_trait_methods,
    reason = "default client handlers are enough for this in-process test"
)]
impl ClientHandler for TestClient {}
#[tokio::test]
async fn openai_style_applies_multiline_patch() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "old\n").unwrap();
    let application = Application::with_database(
        &directory.path().join("history.sqlite3"),
        InputStyle::Openai,
    )
    .unwrap();
    verify_schemas(&application, &["patch"]);
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
fn verify_schemas(application: &Application, expected_properties: &[&str]) {
    let tools = application.tool_router.list_all();
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|tool| tool.output_schema.is_some()));
    for tool in &tools {
        let schema = sonic_rs::to_string(tool.output_schema.as_ref().unwrap()).unwrap();
        assert!(!schema.contains(r#""format":"uint""#));
    }
    let apply = tools
        .iter()
        .find(|tool| tool.name == "apply_patch")
        .unwrap();
    let mut properties = apply
        .input_schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    properties.sort_unstable();
    assert_eq!(properties, expected_properties);
}
async fn call_apply_patch(
    client: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    patch: &str,
) -> rmcp::model::CallToolResult {
    let mut arguments = rmcp::serde_json::Map::new();
    arguments.insert(String::from("patch"), rmcp::serde_json::Value::from(patch));
    let request = ClientRequest::CallToolRequest(Request::new(
        CallToolRequestParams::new("apply_patch").with_arguments(arguments),
    ));
    let result = client.peer().send_request(request).await.unwrap();
    let rmcp::model::ServerResult::CallToolResult(tool_result) = result else {
        panic!("expected call tool result");
    };
    tool_result
}
