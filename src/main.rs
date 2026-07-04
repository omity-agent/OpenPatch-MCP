use apply_patch_mcp::{cli::Cli, server::Application};
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = <Cli as clap::Parser>::parse();
    let application = Application::new();
    let service = rmcp::ServiceExt::serve(application, rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
