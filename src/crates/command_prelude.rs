pub use clap::{value_parser, AppSettings, Arg, ArgAction, ArgMatches};

/// Add a custom flag to subcommand
pub fn flag(name: &'static str, help: &'static str) -> Arg<'static> {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}
