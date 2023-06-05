//! **rustup** subcommand focus on download rustup init files and toolchains from upstream. The core
//! function implemented in the `src/crates/rustup`.
//!
//!
//! **rustup** subcommand provide major functions include:
//!
//!   Arguments:
//!   - __domain__: you can choose your own upstream by adding this argument in command
//!   - __download-threads__: specify the download threads to parallel download,
//!        this param can be changed in the configuration file or pass it here
//!
//! # download subcommand
//!   - sync rustup init from upstream to local
//!   - download subcommand will fetch only the latest version of init file, and this can't be changed by config.
//!   - before each download, freighter will try to fetch the sha256 of the file and compare with local file if it exists
//!         and will skip downloading if they are matching.
//!
//! # upload subcommand
//!   upload file to Object Storage Service compatible with [AWS S3](https://aws.amazon.com/s3/)
//!     - Digitalocean Spaces
//!     - Huawei Cloud OBS
//!     - Alibaba Cloud OSS
//!     - Tencent Cloud COS
//!     - AWS S3
//!     - minio
//!     - Ceph
//!
//!   Arguments:
//!   - __bucket__: set the s3 bucket you want to upload files to, you must provide this param before upload.
//!   

use std::sync::Arc;

use clap::{arg, ArgMatches};
use rayon::ThreadPoolBuilder;

use crate::cloud::s3::S3cmd;
use crate::cloud::CloudStorage;
use crate::commands::command_prelude::*;
use crate::config::Config;
use crate::errors::FreightResult;
use crate::handler::rustup::{sync_rustup_init, RustUpOptions};

pub fn cli() -> clap::Command {
    clap::Command::new("rustup")
        .subcommand(subcommand("download"))
        .subcommand(subcommand("upload")
        .arg(
            arg!(-b --"bucket" <VALUE> "set the s3 bucket you want to upload files to")
            .required(true)
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Sync the Rustup toolchain from the upstream to the local registry")
        .arg(arg!(-t --"download-threads" <VALUE> "specify the download thread count")
            .value_parser(value_parser!(usize))
        )
        .arg(arg!(-d --"domain" <VALUE> "specify the source you want to sync from"))
        .help_template(
            "\
Sync the rustup init files from the upstream(static.rust-lang.org) to the local filesystem, other cloud
storage services, or other registries.

USAGE:
    {usage}

OPTIONS:
{options}

EXAMPLES
1. Download toolchains from source domain(not necessary, default from static.rust-lang.org) 
with 64 download threads

       freighter rustup -t 64 -d https://www.example.com download

2. Upload rustup init file to s3 bucket:

       freighter rustup upload -b bucket-name

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

    crate::cli::init_log(&config.log, work_dir.to_path_buf(), "rustup").unwrap();

    let mut opts = RustUpOptions {
        proxy: config.proxy.to_owned(),
        config: config.rustup.to_owned(),
        rustup_path: work_dir.join("rustup"),
        ..Default::default()
    };

    if let Some(domain) = args.get_one::<String>("domain").cloned() {
        opts.config.domain = domain;
    }

    if let Some(download_threads) = args.get_one::<usize>("download-threads").cloned() {
        opts.config.download_threads = download_threads;
    };

    opts.thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(opts.config.download_threads)
            .build()
            .unwrap(),
    );

    tracing::info!("RustUpOptions info : {:#?}", opts);

    match args.subcommand() {
        Some(("download", _)) => sync_rustup_init(&opts)?,
        Some(("upload", args)) => {
            let bucket_name = args.get_one::<String>("bucket").cloned().unwrap();
            let s3cmd = S3cmd::default();
            s3cmd
                .upload_folder(opts.rustup_path.to_str().unwrap(), &bucket_name)
                .unwrap();
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
