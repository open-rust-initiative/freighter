//! provide common utils for add a flag to clap
use clap::Command;
pub use clap::{value_parser, Arg, ArgAction, ArgMatches};


/// Add a custom flag to subcommand
pub fn flag(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}

pub fn subcommand(name: &'static str) -> Command {
    Command::new(name)
}