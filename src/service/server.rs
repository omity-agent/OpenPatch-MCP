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
mod tests;
