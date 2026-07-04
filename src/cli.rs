use clap::Parser;
#[derive(Debug, Parser)]
#[command(version, about = "MCP server with embedded Codex apply-patch support")]
pub struct Cli;
