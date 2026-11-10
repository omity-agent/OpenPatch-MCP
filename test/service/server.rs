use super::Application;
use crate::service::cli::InputStyle;
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientRequest, Request},
    serde_json::{Value, json},
};
use std::fs;
#[path = "server/paths.rs"]
mod paths;
#[path = "server/structured.rs"]
mod structured;
#[path = "server/undo_protocol.rs"]
mod undo_protocol;
#[derive(Clone, Default)]
struct TestClient;
impl ClientHandler for TestClient {}
type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;
async fn connect(
    application: Application,
) -> (Client, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let server = tokio::spawn(async move {
        ServiceExt::serve(application, server_transport)
            .await?
            .waiting()
            .await?;
        Ok(())
    });
    let client = ServiceExt::serve(TestClient, client_transport)
        .await
        .unwrap();
    (client, server)
}
#[tokio::test]
async fn applies_both_input_styles_and_reports_structured_results() {
    for style in [InputStyle::Openai, InputStyle::General] {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.txt");
        let openai = matches!(style, InputStyle::Openai);
        let original = if openai {
            "old\n"
        } else {
            "before\nold text\nafter\n"
        };
        let expected = if openai {
            "new\n"
        } else {
            "before\nnew text\nafter\n"
        };
        fs::write(&target, original).unwrap();
        let application =
            Application::with_database(&directory.path().join("history.lmdb"), style).unwrap();
        verify_schemas(
            &application,
            if openai {
                &["patch"]
            } else {
                &["new_string", "old_string", "path"]
            },
        );
        let (client, server) = connect(application).await;
        let arguments = if openai {
            json!({"patch": format!(
                "*** Begin Patch\n*** Update File: {}\n@@\n-old\n+new\n*** End Patch\n",
                target.display()
            )})
        } else {
            json!({"path": target, "old_string": "old text", "new_string": "new text"})
        };
        let result = call(&client, "apply_patch", arguments).await;
        assert_eq!(result.is_error, Some(false));
        structured::verify_success(
            result.structured_content.as_ref().unwrap(),
            &target,
            if openai { 1 } else { 3 },
        );
        let text = output_text(&result);
        assert!(text.find("after:").unwrap() < text.find("<ID>").unwrap());
        assert_eq!(fs::read_to_string(&target).unwrap(), expected);
        client.cancel().await.unwrap();
        server.await.unwrap().unwrap();
    }
}
fn verify_schemas(application: &Application, expected_properties: &[&str]) {
    let tools = application.tool_router.list_all();
    assert_eq!(tools.len(), 2);
    for tool in &tools {
        let schema = tool.output_schema.as_ref().unwrap();
        assert!(
            !sonic_rs::to_string(schema)
                .unwrap()
                .contains(r#""format":"uint""#)
        );
        let output = rmcp::serde_json::to_value(schema).unwrap();
        assert_eq!(
            output.pointer("/$defs/OutputOperationKind/enum").unwrap(),
            &json!(["ADD", "EDIT", "DELETE", "MOVE"])
        );
    }
    let undo = tools.iter().find(|tool| tool.name == "undo_patch").unwrap();
    assert_eq!(
        undo.input_schema.as_ref(),
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "properties": {"ids": {"type": "array", "minItems": 1_u64,
                "items": {"type": "string", "pattern": crate::operation::model::ID_PATTERN}}},
            "required": ["ids"], "additionalProperties": false
        })
        .as_object()
        .unwrap()
    );
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
async fn call(client: &Client, name: &str, arguments: Value) -> CallToolResult {
    let request = ClientRequest::CallToolRequest(Request::new(
        CallToolRequestParams::new(name.to_owned())
            .with_arguments(arguments.as_object().unwrap().clone()),
    ));
    let rmcp::model::ServerResult::CallToolResult(result) =
        client.peer().send_request(request).await.unwrap()
    else {
        panic!("expected call tool result");
    };
    result
}
fn output_text(result: &CallToolResult) -> &str {
    result
        .content
        .first()
        .unwrap()
        .as_text()
        .unwrap()
        .text
        .as_str()
}
fn output_value<'result>(result: &'result CallToolResult, pointer: &str) -> &'result Value {
    result
        .structured_content
        .as_ref()
        .unwrap()
        .pointer(pointer)
        .unwrap()
}
