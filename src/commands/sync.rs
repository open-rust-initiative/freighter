///
///
///
///
///

use clap::ArgMatches;

use crate::config::Config;
use crate::errors::FreightResult;
use crate::crates::index::{CrateIndex, SyncOptions, run};
use crate::crates::command_prelude::*;

///
pub fn cli() -> clap::Command<'static> {
    let usage = "freight sync [OPTIONS]";

    clap::Command::new("sync")
        .about("Sync the crates from the upstream(crates.io) to the local registry")
        .usage(usage)
        .arg(flag("no-processbar", "Hide process bar when sync job start"))
        .help_template(
            "\
Sync the crates index and crate files from the upstream(crates.io) to the local filesystem, other cloud
storage services, or other registries.
\n")
}

///
pub fn exec(_config: &mut Config, args: &ArgMatches) -> FreightResult {

    println!("start sync...");
    let index = CrateIndex::default();

    run(index, &mut SyncOptions{
        no_processbar: args.get_flag("no-processbar"),
    })?;

    Ok(())
}