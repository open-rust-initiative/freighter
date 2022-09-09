///
///
///
///
///

use clap::ArgMatches;

use crate::config::Config;
use crate::errors::FreightResult;
use crate::crates::index::{CrateIndex, run};
use crate::crates::command_prelude::*;

///
pub fn cli() -> clap::Command<'static> {
    let usage = "freight sync [OPTIONS]";

    clap::Command::new("sync")
        .about("Sync the crates from the upstream(crates.io) to the local registry")
        .usage(usage)
        .arg(flag("show-processbar", "Show process bar when sync job start"))
        .help_template(
            "\
Sync the crates index and crate files from the upstream(crates.io) to the local filesystem, other cloud
storage services, or other registries.
\n")
}

///
pub fn exec(_config: &mut Config, args: &ArgMatches) -> FreightResult {
    println!("set show-processbar value:{}", args.get_flag("show-processbar"));

    println!("start sync...");
    let index = CrateIndex::default();
    if let Err(e) = run(index) {
        e.print();
    }

    Ok(())
}