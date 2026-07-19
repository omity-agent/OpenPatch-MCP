use crate::command::{PatchExecution, PatchRunner, UndoExecution};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
#[derive(Clone)]
pub struct Application {
    tool_router: ToolRouter<Self>,
    runner: PatchRunner,
}
#[derive(Debug, Deserialize, schemars :: JsonSchema)]
pub struct ApplyPatchRequest {
    pub patch: String,
}
#[derive(Debug, Deserialize, schemars :: JsonSchema)]
pub struct UndoPatchRequest {
    pub uuids: Vec<String>,
}
#[tool_router]
impl Application {
    #[inline]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(),
            runner: PatchRunner::open_default()?,
        })
    }
    #[cfg(test)]
    fn with_database(path: &std::path::Path) -> anyhow::Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(),
            runner: PatchRunner::open(path)?,
        })
    }
    # [tool (name = "apply_patch" , description = "The `apply_patch` tool can be used to edit files. Each patch will be assigned a UUID. This is a FREEFORM tool, so do not wrap the patch in JSON." , output_schema = rmcp :: handler :: server :: tool :: schema_for_type ::< crate :: operation :: PatchToolOutput > ())]
    async fn apply_patch(
        &self,
        Parameters(request): Parameters<ApplyPatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let output = self.runner.apply(PatchExecution {
            patch: &request.patch,
        });
        Ok(to_tool_result(&output))
    }
    # [tool (name = "undo_patch" , description = "Undo recorded patch operations. When you want to undo changes, always use the `undo_patch` tool instead of manually rewriting them. The tool is more efficient and ensures that the undoed content are exactly the same as the original." , output_schema = rmcp :: handler :: server :: tool :: schema_for_type ::< crate :: operation :: PatchToolOutput > ())]
    async fn undo_patch(
        &self,
        Parameters(request): Parameters<UndoPatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let output = self.runner.undo(UndoExecution {
            uuids: &request.uuids,
        });
        Ok(to_tool_result(&output))
    }
}
fn to_tool_result(output: &crate::command::PatchOutput) -> CallToolResult {
    let succeeded = output.succeeded();
    let content = vec![ContentBlock::text(output.render().to_owned())];
    let mut result = if succeeded {
        CallToolResult::success(content)
    } else {
        CallToolResult::error(content)
    };
    result.structured_content = Some(output.structured().clone());
    result
}
# [tool_handler (router = self . tool_router)]
#[expect(
    clippy::missing_trait_methods,
    clippy::unused_async_trait_impl,
    reason = "rmcp supplies default handlers and generated async glue"
)]
impl ServerHandler for Application {}
#[cfg(test)]
mod tests {
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
        let database_path = directory.path().join("history.sqlite3");
        let application = Application::with_database(&database_path).unwrap();
        let tools = application.tool_router.list_all();
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().all(|tool| tool.output_schema.is_some()));
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
        let arguments = rmcp::model::object(rmcp :: serde_json :: json ! ({ "patch" : patch }));
        let request = ClientRequest::CallToolRequest(Request::new(
            CallToolRequestParams::new("apply_patch").with_arguments(arguments),
        ));
        let result = client.peer().send_request(request).await.unwrap();
        let rmcp::model::ServerResult::CallToolResult(tool_result) = result else {
            panic!("expected call tool result");
        };
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
        let structured = tool_result.structured_content.unwrap();
        let result_object = structured.as_object().unwrap();
        assert_eq!(
            result_object.get("succeeded"),
            Some(&rmcp::serde_json::json!(true))
        );
        let successes = result_object.get("successes").unwrap().as_array().unwrap();
        let success = successes.first().unwrap().as_object().unwrap();
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
        assert!(
            result_object
                .get("failures")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "new\n");
        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }
}
