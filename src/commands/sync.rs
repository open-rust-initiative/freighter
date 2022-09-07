///
///
///
///
///

use clap::ArgMatches;

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::errors::FreightResult;
use crate::crates::index::CrateIndex;

///
pub fn cli() -> clap::Command<'static> {
    let usage = "freight sync [OPTIONS]";

    clap::Command::new("sync")
        .about("Sync the crates from the upstream(crates.io) to the local registry")
        .usage(usage)
        .help_template(
            "\
Sync the crates index and crate files from the upstream(crates.io) to the local filesystem, other cloud
storage services, or other registries.
\n")
}

///
pub fn exec(_config: &mut Config, args: &ArgMatches) -> FreightResult {
    Ok(())
}