use super::Application;
use crate::service::cli::InputStyle;
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, CallToolResult},
    serde_json::{Value, json},
};
use std::{fs, path::Path};
#[path = "server/paths.rs"]
mod paths;
#[path = "server/undo_protocol.rs"]
mod undo_protocol;
#[derive(Clone, Default)]
struct TestClient;
impl ClientHandler for TestClient {}
type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;
async fn connect(
    directory: &tempfile::TempDir,
    style: InputStyle,
) -> (Client, tokio::task::JoinHandle<anyhow::Result<()>>) {
    let application =
        Application::with_database(&directory.path().join("history.lmdb"), style).unwrap();
    verify_schemas(
        &application,
        match style {
            InputStyle::Openai => &["patch"],
            InputStyle::General => &["new_string", "old_string", "path"],
        },
    );
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
    for (style, original, expected, changes) in [
        (InputStyle::Openai, "old\n", "new\n", "@@\n-old\n+new"),
        (InputStyle::Openai, "old\n", "new\n", "-old\n+new"),
        (
            InputStyle::Openai,
            "old\n\nkeep\n",
            "new\n\nkeep\n",
            "@@\n-old\n+new\n\n keep",
        ),
        (
            InputStyle::Openai,
            "old\n",
            "new\n",
            "@@  \n-old\n+new\n*** End of File  \n\n",
        ),
        (
            InputStyle::General,
            "before\nold text\nafter\n",
            "before\nnew text\nafter\n",
            "",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.txt");
        let openai = matches!(style, InputStyle::Openai);
        fs::write(&target, original).unwrap();
        let (client, server) = connect(&directory, style).await;
        let arguments = if openai {
            json!({"patch": format!(
                "*** Begin Patch\n*** Update File: {}\n{changes}\n*** End Patch\n",
                target.display()
            )})
        } else {
            json!({"path": target, "old_string": "old text", "new_string": "new text"})
        };
        let result = call(&client, "apply_patch", arguments).await;
        assert_eq!(result.is_error, Some(false));
        verify_success(
            result.structured_content.as_ref().unwrap(),
            &target,
            u64::try_from(original.lines().count()).unwrap(),
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
fn verify_success(structured: &Value, target: &Path, line_count: u64) {
    assert_eq!(structured["succeeded"], true);
    let success = structured.pointer("/successes/0").unwrap();
    assert_eq!(success.as_object().unwrap().len(), 6);
    assert_eq!(success["kind"], "EDIT");
    assert_eq!(
        success["path"].as_str().unwrap(),
        target.file_name().unwrap().to_string_lossy()
    );
    for state in ["before", "after"] {
        assert_eq!(
            *success.pointer(&format!("/{state}/lineCount")).unwrap(),
            line_count
        );
    }
    let id = success["id"].as_str().unwrap();
    crate::operation::model::OperationId::parse(id).unwrap();
    assert!(success["undoOf"].is_null());
    assert_eq!(structured["failures"], Value::Array(Vec::new()));
}
async fn call(client: &Client, name: &str, arguments: Value) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(arguments.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}
fn output_text(result: &CallToolResult) -> &str {
    result.content[0].as_text().unwrap().text.as_str()
}
fn output_value<'result>(result: &'result CallToolResult, pointer: &str) -> &'result Value {
    result
        .structured_content
        .as_ref()
        .unwrap()
        .pointer(pointer)
        .unwrap()
}
