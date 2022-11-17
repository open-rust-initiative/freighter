//! **rustup** subcommand focus on the sync rustup init and rust toolchains from upstream. The core
//! function implemented in the `src/crates/rustup`.
//!
//! **sync** subcommand provide major functions include:
//! - sync rustup init from upstream to local
//! - sync rust toolchains file from upstream to local
//! - sync file to Object Storage Service compatible with [AWS S3](https://aws.amazon.com/s3/)
//!     - Digitalocean Spaces
//!     - Huawei Cloud OBS
//!     - Alibaba Cloud OSS
//!     - Tencent Cloud COS
//!     - AWS S3
//!     - minio
//!     - Ceph
//!

use clap::{arg, ArgMatches};
use log::info;

use crate::config::Config;
use crate::crates::command_prelude::*;
use crate::crates::rustup::{sync_rustup, upload_to_s3, RustUpOptions};
use crate::errors::FreightResult;

pub fn cli() -> clap::Command {
    clap::Command::new("rustup")
        .subcommand(subcommand("download")
            .arg(flag("clean", "clean up historical versions"))
            .arg(arg!(-v --"version" <VALUE> "download only specified version"))
        )
        .subcommand(subcommand("upload")
        .arg(
            arg!(-b --"bucket" <VALUE> "set the s3 bucket you want to upload files")
            .required(true)
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Sync the Rustup toolchain from the upstream to the local registry")
        .arg(flag("no-progressbar", "Hide progressbar when start sync"))
        .arg(arg!(-t --"download-threads" <VALUE> "specify the download thread count")
            .value_parser(value_parser!(usize))
        )
        .arg(arg!(-d --"domain" <VALUE> "specify the source you want to sync from"))
        .help_template(
            "\
Sync the rust toolchian files from the upstream(static.rust-lang.org) to the local filesystem, other cloud
storage services, or other registries.

USAGE:
    {usage}

OPTIONS:
{options}

EXAMPLES
1. Download toolchains from source domain(not necessary, default from static.rust-lang.org) 
with 64 download threads and then clean historical files

       freighter rustup -t 64 -d https://www.example.com download --clean

2. Upload rustup init file and toolchains to s3 bucket:

       freighter rustup upload -b bucket-name

3. Download specify version:

       freighter rustup download -v nightly-2022-07-31

\n")
}

///
///
///
pub fn exec(config: &mut Config, args: &ArgMatches) -> FreightResult {
    let work_dir = config
        .work_dir
        .as_ref()
        .expect("something bad happened because work_dir is none");

    let mut opts = RustUpOptions {
        config: config.rustup.to_owned(),
        no_progressbar: args.get_flag("no-progressbar"),
        rustup_path: work_dir.join("freighter/rustup"),
        dist_path: work_dir.join("freighter/dist"),
        ..Default::default()
    };

    if let Some(domain) = args.get_one::<String>("domain").cloned() {
        opts.config.domain = domain;
    }

    if let Some(download_threads) = args.get_one::<usize>("download-threads").cloned() {
        opts.config.download_threads = download_threads;
    };

    info!("RustUpOptions info : {:#?}", opts);

    match args.subcommand() {
        Some(("download", args)) => sync_rustup(&RustUpOptions {
            clean: args.get_flag("clean"),
            version: args.get_one::<String>("version").cloned(),
            ..opts
        })?,
        Some(("upload", args)) => {
            opts.bucket_name = args.get_one::<String>("bucket").cloned().unwrap();
            upload_to_s3(&opts)?
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

