use clap::Parser;
use std::path::PathBuf;
#[derive(Debug, Parser)]
#[command(version, about = "MCP wrapper around the Codex apply-patch executable")]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,
}
