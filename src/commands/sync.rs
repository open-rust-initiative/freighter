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

use std::path::PathBuf;

use clap::{arg, ArgMatches};

use crate::config::Config;
use crate::crates::command_prelude::*;
use crate::crates::index::{download, pull, upload_to_s3, CrateIndex, SyncOptions};
use crate::crates::rustup::sync_rustup;
use crate::errors::FreightResult;

/// The __sync__ subcommand
///

pub fn cli() -> clap::Command {
    clap::Command::new("sync")
        .subcommand(subcommand("pull"))
        .subcommand(subcommand("rustup")
            .arg(flag("clean", "clean up historical files")))
        .subcommand(subcommand("upload")
        .arg(
            arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files")
            .required(true)
        ))
        .subcommand(subcommand("download")
            .arg(flag("init", "Start init download of crates file, this will traverse all index for full download"))
            .arg(flag("upload", "upload file after download"))
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Sync the crates from the upstream(crates.io) to the local registry")
        .arg(flag("no-progressbar", "Hide progressbar when start sync"))
        .arg(
            arg!(-c --"work-dir" <FILE> "specify the work dir, which contains crates path, index path and log path, default: $HOME/.freighter")
            .required(false)
        )
        .arg(
            arg!(-t --"thread-count" <VALUE> "specify the download thread count, default will be 16")
            .value_parser(value_parser!(usize))
            .required(false)
        )
        .help_template(
            "\
Sync the crates index and crate files from the upstream(crates.io) to the local filesystem, other cloud
storage services, or other registries.

USAGE:
    {usage}

OPTIONS:
{options}

EXAMPLES
1. Sync the crates index with specify directory

       freighter sync -c /mnt/volume_fra1_01 pull

2. Download all crates file and uoload:

       freighter sync download --init --upload

3. Download crates file with multi-thread to specify directory:

       freighter sync -t 512 -c /mnt/volume_fra1_01 download --init

\n")
}

///
///
///
pub fn exec(_config: &mut Config, args: &ArgMatches) -> FreightResult {
    let mut index = match args.get_one::<String>("work-dir").cloned() {
        Some(work_dir) => CrateIndex::new(PathBuf::from(work_dir)),
        None => {
            let index: CrateIndex = Default::default();
            println!("use default crates path: {}", index.crates_path.display());
            index
        }
    };

    match args.get_one::<usize>("thread-count").cloned() {
        Some(thread_count) => index.thread_count = thread_count,
        None => println!("use default thread count: {}", index.thread_count),
    };

    println!("CrateIndex info : {:#?}", index);
    let no_progressbar = args.get_flag("no-progressbar");

    match args.subcommand() {
        Some(("pull", _args)) => pull(
            index,
            &mut SyncOptions {
                no_progressbar,
                init: false,
            },
        )?,
        Some(("download", args)) => {
            index.upload = args.get_flag("upload");
            download(
                index,
                &mut SyncOptions {
                    no_progressbar,
                    init: args.get_flag("init"),
                },
            )?
        }
        Some(("rustup", args)) => {
            sync_rustup(index, args.get_flag("clean"))?
        },
        Some(("upload", args)) => {
            let bucket = args.get_one::<String>("bucket").cloned().unwrap();
            upload_to_s3(index, &bucket)?
        }
        Some((cmd, _)) => {
            unreachable!("unexpected command {}", cmd)
        }
        None => {
            unreachable!("unexpected command")
        }
    };

    Ok(())
}
