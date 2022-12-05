//! **channel** subcommand focus on download rust toolchains from upstream. The core
//! function implemented in the `src/crates/channel`.
//!
//!
//! **channel** subcommand provide major functions include:
//!
//!   Arguments:
//!   - __domain__: you can choose your own upstream by adding this argument in command
//!   - __download-threads__: specify the download threads to parallel download, 
//!        this param can be changed in the configuration file or pass it here
//!   - __no-progressbar__: not implemented
//!
//! # download subcommand
//!   - before each download, freighter will try to fetch the sha256 of the file and compare with local file if it exists
//!         and will skip downloading if they are matching.
//!   
//!   - sync server rust toolchains version from upstream to local
//!     - by default, this subcommand will fetch latest stable, beta, nightly and
//!         the specified version in your toml config file: __rustup.sync_stable_versions__
//!     - if you are using --version arguments in subcommand, freighter will only download the version you specified,
//!         learn more about [rust release info](https://forge.rust-lang.org/index.html) here
//!     - in the download process, freighter will first download the channel file, for example: channel-rust-1.29.toml
//!
//!   Arguments:
//!   - __clean__: clean history files read by config file after download successfully.
//!   - __version__: only download the version you specified,
//!         you can provide any version format supported by rust-org, such as stable, beta or nightly-2022-07-31.
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

use clap::{arg, ArgMatches};
use log::info;

use crate::cloud::s3::{CloudStorage, S3cmd};
use crate::commands::command_prelude::*;
use crate::config::Config;
use crate::crates::channel::{sync_rust_toolchain, ChannelOptions};
use crate::errors::FreightResult;

pub fn cli() -> clap::Command {
    clap::Command::new("channel")
        .subcommand(subcommand("download")
            .arg(flag("clean", "clean up historical versions"))
            .arg(arg!(-v --"version" <VALUE> "only download the version you specified"))
            .arg(flag("upload", "upload every crate file after download"))
            .arg(arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files"))
            .arg(flag("delete-after-upload", "this will delete file after upload"))
        )
        .subcommand(subcommand("upload")
        .arg(
            arg!(-b --"bucket" <VALUE> "set the s3 bucket you want to upload files to")
            .required(true)
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Sync the Rust toolchain from the upstream to the local registry")
        .arg(flag("no-progressbar", "Hide progressbar when start sync"))
        .arg(arg!(-t --"download-threads" <VALUE> "specify the download thread count")
            .value_parser(value_parser!(usize))
        )
        .arg(arg!(-d --"domain" <VALUE> "specify the source you want to sync from"))
        .help_template(
            "\
Sync the rust toolchain files from the upstream(static.rust-lang.org) to the local filesystem, other cloud
storage services, or other registries.

USAGE:
    {usage}

OPTIONS:
{options}

EXAMPLES
1. Download toolchains from source domain(not necessary, default from static.rust-lang.org) 
with 64 download threads and then clean historical files

       freighter channel -t 64 -d https://www.example.com download --clean

2. Upload rust toolchains to s3 bucket:

       freighter channel upload -b bucket-name

3. Download specify version:

       freighter channel download -v nightly-2022-07-31

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

    crate::cli::init_log(&config.log, work_dir.to_path_buf(), "channel").unwrap();

    let mut opts = ChannelOptions {
        config: config.rustup.to_owned(),
        dist_path: work_dir.join("dist"),
        ..Default::default()
    };

    if let Some(domain) = args.get_one::<String>("domain").cloned() {
        opts.config.domain = domain;
    }

    if let Some(download_threads) = args.get_one::<usize>("download-threads").cloned() {
        opts.config.download_threads = download_threads;
    };

    info!("ChannelOptions info : {:#?}", opts);

    match args.subcommand() {
        Some(("download", args)) => {
            let bucket_name = args.get_one::<String>("bucket").cloned();
            let upload = args.get_flag("upload");
            if upload && bucket_name.is_none() {
                unreachable!("can not upload with empty bucket name")
            }

            sync_rust_toolchain(&ChannelOptions {
                clean: args.get_flag("clean"),
                version: args.get_one::<String>("version").cloned(),
                upload,
                delete_after_upload: args.get_flag("delete-after-upload"),
                bucket_name: bucket_name.unwrap(),
                ..opts
            })?
        },
        Some(("upload", args)) => {
            let bucket_name = args.get_one::<String>("bucket").cloned().unwrap();
            let s3cmd = S3cmd::default();
            s3cmd
                .upload_folder(opts.dist_path.to_str().unwrap(), &bucket_name)
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
