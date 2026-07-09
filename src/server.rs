use crate::command::{PatchExecution, PatchRunner};
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
}
#[derive(Debug, Deserialize, schemars :: JsonSchema)]
pub struct ApplyPatchRequest {
    pub patch: String,
}
#[tool_router]
impl Application {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
    #[tool(
        name = "apply_patch",
        description = "Use the `apply_patch` tool to edit files. This is a FREEFORM tool, so do not wrap the patch in JSON."
    )]
    async fn apply_patch(
        &self,
        Parameters(request): Parameters<ApplyPatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        if request.patch.trim().is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "patch must not be empty",
            )]));
        }
        let output = PatchRunner::apply(PatchExecution {
            patch: &request.patch,
        });
        let content = vec![ContentBlock::text(output.render())];
        if output.succeeded() {
            Ok(CallToolResult::success(content))
        } else {
            Ok(CallToolResult::error(content))
        }
    }
}
impl Default for Application {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
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
    async fn mcp_call_applies_multiline_patch_with_embedded_runner() {
        let directory = tempfile::tempdir().unwrap();
        let target_path = directory.path().join("target.txt");
        fs::write(&target_path, "old\n").unwrap();
        let application = Application::new();
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
        assert_eq!(fs::read_to_string(&target_path).unwrap(), "new\n");
        client.cancel().await.unwrap();
        server_handle.await.unwrap().unwrap();
    }
}
