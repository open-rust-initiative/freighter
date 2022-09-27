//! **pull** subcommand focus on the pull crates index from upstream. The core
//! function implemented in the `src/crates/index`.
//!


use clap::ArgMatches;

use crate::config::Config;
use crate::errors::FreightResult;
use crate::crates::index::{CrateIndex, SyncOptions, pull};
use crate::crates::command_prelude::*;

/// The __pull__ subcommand
///
///
pub fn cli() -> clap::Command<'static> {
    let usage = "freight pull [OPTIONS]";

    clap::Command::new("pull")
        .about("Pull the crates from the upstream(crates.io) to the local registry")
        .usage(usage)
        .arg(flag("no-progressbar", "Hide process bar when start pull"))
        .help_template(
            "\
Pull the crates index from the upstream(crates.io) to the local filesystem, other cloud
storage services, or other registries.

OPTIONS:
{options}

\n")
}

///
///
///
pub fn exec(_config: &mut Config, args: &ArgMatches) -> FreightResult {
    let index = CrateIndex::default();

    pull(index, &mut SyncOptions{
        no_progressbar: args.get_flag("no-progressbar"),
    })?;

    Ok(())
}