//! **sync** subcommand focus on the sync crates index and crate files from upstream. The core
//! function implemented in the `src/crates/index`.
//!
//! **sync** subcommand provide major functions include:
//! - sync crates index from upstream to local
//!     - The crates index is a git repository, and **cargo** clone and update from [GitHub](https://github.com/rust-lang/crates.io-index).
//!         - The clone use `bare` mode, more details in the [cargo guide](https://github.com/rust-lang/cargo/blob/6b6b0b486d73c03ed952591d880debec1d47c534/src/doc/src/guide/cargo-home.md#directories)
//! - sync crate file from upstream to local
//!    - The crate file of upstream location detail from [crates.io-index](https://github.com/rust-lang/crates.io-index/blob/master/.github/workflows/update-dl-url.yml)
//!      ```YAML
//!      env:
//!         URL_api: "https://crates.io/api/v1/crates"
//!         URL_cdn: "https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
//!         URL_s3_primary: "https://crates-io.s3-us-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
//!         URL_s3_fallback: "https://crates-io-fallback.s3-eu-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
//!      ```
//! - sync crate file to Object Storage Service compatible with [AWS S3](https://aws.amazon.com/s3/)
//!     - Digitalocean Spaces
//!     - Huawei Cloud OBS
//!     - Alibaba Cloud OSS
//!     - Tencent Cloud COS
//!     - AWS S3
//!     - minio
//!     - Ceph
//!

use clap::ArgMatches;

use crate::config::Config;
use crate::errors::FreightResult;
use crate::crates::index::{CrateIndex, SyncOptions, run};
use crate::crates::command_prelude::*;

/// The __sync__ subcommand
///
///
pub fn cli() -> clap::Command<'static> {
    let usage = "freight sync [OPTIONS]";

    clap::Command::new("sync")
        .about("Sync the crates from the upstream(crates.io) to the local registry")
        .usage(usage)
        .arg(flag("no-progressbar", "Hide process bar when start sync"))
        .help_template(
            "\
Sync the crates index and crate files from the upstream(crates.io) to the local filesystem, other cloud
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

    run(index, &mut SyncOptions{
        no_processbar: args.get_flag("no-progressbar"),
    })?;

    Ok(())
}