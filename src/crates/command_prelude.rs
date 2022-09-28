use clap::App;
pub use clap::{value_parser, AppSettings, Arg, ArgAction, ArgMatches};

/// Add a custom flag to subcommand
pub fn flag(name: &'static str, help: &'static str) -> Arg<'static> {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}

pub fn subcommand(name: &'static str) -> App {
    App::new(name)
        .dont_collapse_args_in_usage(true)
        .setting(AppSettings::DeriveDisplayOrder)
}