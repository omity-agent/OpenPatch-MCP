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
mod tests {
    use super::{Cli, InputStyle};
    use clap::Parser as _;
    #[test]
    fn general_is_the_default_style() {
        let cli = Cli::try_parse_from(["openpatch"]).unwrap();
        assert_eq!(cli.style, InputStyle::General);
    }
    #[test]
    fn parses_both_style_values() {
        let general = Cli::try_parse_from(["openpatch", "--style=general"]).unwrap();
        let openai = Cli::try_parse_from(["openpatch", "--style=openai"]).unwrap();
        assert_eq!(general.style, InputStyle::General);
        assert_eq!(openai.style, InputStyle::Openai);
    }
}
