use clap::{Parser, ValueEnum};
#[derive(Debug, Parser)]
#[command(version, about = "MCP server that applies and records file edits")]
pub struct Cli {
    #[arg(long, value_enum, default_value_t)]
    pub style: InputStyle,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
#[non_exhaustive]
pub enum InputStyle {
    #[default]
    General,
    Openai,
}
#[cfg(test)]
#[path = "../../test/service/cli.rs"]
mod tests;
