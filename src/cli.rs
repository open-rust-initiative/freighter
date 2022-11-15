//!
//!
//!
//!
//!

use std::path::PathBuf;

use clap::{arg, ArgMatches};

use crate::commands;
use crate::config::Config;
use crate::errors::{FreightResult, FreighterError};

///
///
///
///
///

pub type App = clap::Command;

///
///
pub fn main(config: &mut Config) -> FreightResult {
    // let mut config = Config::new();

    let args = cli().try_get_matches()?;
    // let cmd = args.subcommand_name().unwrap();

    let work_dir = match args.get_one::<String>("work-dir").cloned() {
        Some(work_dir) => PathBuf::from(work_dir),
        None => dirs::home_dir().unwrap(),
    };
    let mut config = config.load(&work_dir);
    config.work_dir = Some(work_dir);

    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            cli().print_help()?;
            return Ok(());
        }
    };

    execute_subcommand(&mut config, cmd, subcommand_args)
}

///
///
fn cli() -> App {
    let usage = "freight [SUBCOMMAND]";

    App::new("freight")
        .version("0.1.0")
        .disable_colored_help(true)
        .disable_help_subcommand(true)
        .override_usage(usage)
        .author("Open Rust Initiative")
        .arg(arg!(-c --"work-dir" <FILE> "specify the work dir,
             where to downlaod crates, rust toolchains and storage logs, default: $HOME/.freighter")
        )
        .help_template(
            "\
Freight - A crate registry from the Open Rust Initiative Community

USAGE:
    {usage}

Some common freight commands are (see all commands with --list):
    sync    Sync the index and crate files from the upstream to local, cloud or registry

See 'freight help <command>' for more information on a specific command.\n"
        )
        .subcommands(commands::builtin())
}

///
///
pub fn execute_subcommand(config: &mut Config, cmd: &str, args: &ArgMatches) -> FreightResult {
    if let Some(f) = commands::builtin_exec(cmd) {
        f(config, args)
    } else {
        Err(FreighterError::unknown_command(cmd.to_string()))
    }
}
