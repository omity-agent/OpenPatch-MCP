use crate::{
    command::{PatchExecution, PatchRunner, ReplaceExecution, UndoExecution},
    service::cli::InputStyle,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        router::tool::{ToolRoute, ToolRouter},
        tool::{schema_for_input, schema_for_type},
        wrapper::Parameters,
    },
    model::{CallToolResult, ContentBlock, Tool},
    schemars, tool, tool_handler,
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
pub struct ReplaceRequest {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
}
#[derive(Debug, Deserialize, schemars :: JsonSchema)]
pub struct UndoPatchRequest {
    pub uuids: Vec<String>,
}
impl Application {
    #[inline]
    pub fn new(style: InputStyle) -> anyhow::Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(style)?,
            runner: PatchRunner::open_default()?,
        })
    }
    #[cfg(test)]
    fn with_database(path: &std::path::Path, style: InputStyle) -> anyhow::Result<Self> {
        Ok(Self {
            tool_router: Self::tool_router(style)?,
            runner: PatchRunner::open(path)?,
        })
    }
    fn tool_router(style: InputStyle) -> anyhow::Result<ToolRouter<Self>> {
        let mut router =
            ToolRouter::new().with_route((Self::undo_patch_tool_attr(), Self::undo_patch));
        add_apply_route(&mut router, style)?;
        Ok(router)
    }
    fn apply_openai(
        &self,
        Parameters(request): Parameters<ApplyPatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let output = self.runner.apply(PatchExecution {
            patch: &request.patch,
        });
        to_tool_result(&output)
    }
    fn apply_general(
        &self,
        Parameters(request): Parameters<ReplaceRequest>,
    ) -> Result<CallToolResult, McpError> {
        let output = self.runner.replace(ReplaceExecution {
            path: &request.path,
            old_string: &request.old_string,
            new_string: &request.new_string,
        });
        to_tool_result(&output)
    }
    # [tool (name = "undo_patch" , description = "Undo recorded patch operations. When you want to undo changes, always use the `undo_patch` tool instead of manually rewriting them. The tool is more efficient and ensures that the undoed content are exactly the same as the original." , output_schema = rmcp :: handler :: server :: tool :: schema_for_type ::< crate :: operation :: PatchToolOutput > ())]
    async fn undo_patch(
        &self,
        Parameters(request): Parameters<UndoPatchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let output = self.runner.undo(UndoExecution {
            uuids: &request.uuids,
        });
        to_tool_result(&output)
    }
}
fn add_apply_route(router: &mut ToolRouter<Application>, style: InputStyle) -> anyhow::Result<()> {
    match style { InputStyle :: General => router . add_route (ToolRoute :: new (apply_tool :: < ReplaceRequest > ("Replace old_string in path with new_string using exact and fuzzy matching. Each edit is assigned a UUID." ,) ? , Application :: apply_general ,)) , InputStyle :: Openai => router . add_route (ToolRoute :: new (apply_tool :: < ApplyPatchRequest > ("The `apply_patch` tool can be used to edit files. Each patch will be assigned a UUID. This is a FREEFORM tool, so do not wrap the patch in JSON." ,) ? , Application :: apply_openai ,)) , }
    Ok(())
}
fn apply_tool<Request>(description: &'static str) -> anyhow::Result<Tool>
where
    Request: schemars::JsonSchema + 'static,
{
    let input_schema = schema_for_input::<Request>().map_err(anyhow::Error::msg)?;
    Ok(Tool::new("apply_patch", description, input_schema)
        .with_raw_output_schema(schema_for_type::<crate::operation::PatchToolOutput>()))
}
fn to_tool_result(output: &crate::command::PatchOutput) -> Result<CallToolResult, McpError> {
    let succeeded = output.succeeded();
    let content = vec![ContentBlock::text(output.render().to_owned())];
    let structured_content = rmcp::serde_json::Value::deserialize(output.structured())
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    let mut result = if succeeded {
        CallToolResult::success(content)
    } else {
        CallToolResult::error(content)
    };
    result.structured_content = Some(structured_content);
    Ok(result)
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
