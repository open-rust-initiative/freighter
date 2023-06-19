//! The commands mod is a fork of [**cargo** commands struct](https://github.com/rust-lang/cargo/blob/master/src/bin/cargo/commands/mod.rs).
//!
//!
//!
//!

use clap::ArgMatches;

use crate::cli::App;
use crate::config::Config;
use crate::errors::FreightResult;

pub mod channel;
pub mod command_prelude;
pub mod crates;
pub mod rustup_init;
pub mod server;

/// The builtin function is the entry point of commands mod. Each subcommand is a
/// `clap::Command<'static>` type, and the `exec` function is logic entry.
/// Add the subcommand to the `Vec<clap::Command<'static>>` and will list in the **freighter**
/// subcommands.
///
/// Each subcommand is a mod in the `src/commands` directory, the `cli` function is the entry
/// point and the `exec` function is logic entry.
///
pub fn builtin() -> Vec<App> {
    vec![
        crates::cli(),
        rustup_init::cli(),
        channel::cli(),
        server::cli(),
    ]
}

///
///
///
pub fn builtin_exec(cmd: &str) -> Option<fn(&mut Config, &ArgMatches) -> FreightResult> {
    let f = match cmd {
        "crates" => crates::exec,
        "rustup" => rustup_init::exec,
        "channel" => channel::exec,
        "server" => server::exec,
        _ => return None,
    };

    Some(f)
}
