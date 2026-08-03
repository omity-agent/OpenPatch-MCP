use super::{Application, InputStyle, TestClient, verify_schemas};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientRequest, Request},
};
use std::{fs, path::Path};
#[tokio::test]
async fn replaces_a_string() {
    let directory = tempfile::tempdir().unwrap();
    let target_path = directory.path().join("target.txt");
    fs::write(&target_path, "before\nold text\nafter\n").unwrap();
    let application = Application::with_database(
        &directory.path().join("history.sqlite3"),
        InputStyle::General,
    )
    .unwrap();
    verify_schemas(&application, &["new_string", "old_string", "path"]);
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let server_handle = tokio::spawn(async move {
        let service = ServiceExt::serve(application, server_transport).await?;
        service.waiting().await?;
        anyhow::Ok(())
    });
    let client = ServiceExt::serve(TestClient, client_transport)
        .await
        .unwrap();
    let tool_result = call_replace(&client, &target_path, "old text", "new text").await;
    assert_eq!(tool_result.is_error, Some(false));
    verify_replacement_success(&tool_result.structured_content.unwrap(), &target_path);
    assert_eq!(
        fs::read_to_string(&target_path).unwrap(),
        "before\nnew text\nafter\n"
    );
    client.cancel().await.unwrap();
    server_handle.await.unwrap().unwrap();
}
fn verify_replacement_success(structured: &rmcp::serde_json::Value, target_path: &Path) {
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
        .unwrap();
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
        Some(3)
    );
    assert_eq!(
        success
            .get("after")
            .unwrap()
            .get("lineCount")
            .unwrap()
            .as_u64(),
        Some(3)
    );
    assert!(success.get("uuid").unwrap().is_string());
}
async fn call_replace(
    client: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    path: &Path,
    old_string: &str,
    new_string: &str,
) -> rmcp::model::CallToolResult {
    let arguments = rmcp::model::object(
        rmcp :: serde_json :: json ! ({ "path" : path . display () . to_string () , "old_string" : old_string , "new_string" : new_string , }),
    );
    let request = ClientRequest::CallToolRequest(Request::new(
        CallToolRequestParams::new("apply_patch").with_arguments(arguments),
    ));
    let result = client.peer().send_request(request).await.unwrap();
    let rmcp::model::ServerResult::CallToolResult(tool_result) = result else {
        panic!("expected call tool result");
    };
    tool_result
}
