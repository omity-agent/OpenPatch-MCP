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
