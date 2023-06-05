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

use std::sync::Arc;

use clap::{arg, ArgMatches};
use rayon::ThreadPoolBuilder;

use crate::cloud;
use crate::cloud::s3::S3cmd;
use crate::commands::command_prelude::*;
use crate::config::Config;
use crate::errors::FreightResult;
use crate::handler::channel::{sync_rust_toolchain, ChannelOptions};

pub fn cli() -> clap::Command {
    clap::Command::new("channel")
        .subcommand(subcommand("download")
            .arg(flag("clean", "clean up historical versions"))
            .arg(arg!(-v --"version" <VALUE> "only download the version you specified"))
            .arg(flag("init", "this command will download the histoey release stable version which you matain in your config file"))
            .arg(flag("upload", "upload every crate file after download"))
            .arg(flag("history", "only sync history nightly and beta versions"))
            .arg(arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files"))
            .arg(flag("delete-after-upload", "this will delete file after upload"))
        )
        .subcommand(subcommand("upload")
            .arg(arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files")
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
        proxy: config.proxy.to_owned(),
        dist_path: work_dir.join("dist"),
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

    tracing::info!("Default ChannelOptions : {:#?}", opts);

    match args.subcommand() {
        Some(("download", args)) => {
            let down_opts = &ChannelOptions {
                upload: args.get_flag("upload"),
                bucket: args.get_one::<String>("bucket").cloned(),
                clean: args.get_flag("clean"),
                version: args.get_one::<String>("version").cloned(),
                delete_after_upload: args.get_flag("delete-after-upload"),
                sync_history: args.get_flag("history"),
                init: args.get_flag("init"),
                ..opts
            };
            if down_opts.upload && down_opts.bucket.is_none() {
                unreachable!("can not upload with empty bucket name")
            }
            sync_rust_toolchain(down_opts)?
        }
        Some(("upload", args)) => {
            let bucket_name = args.get_one::<String>("bucket").cloned().unwrap();
            let s3cmd = S3cmd::default();
            cloud::upload_with_pool(opts.dist_path, opts.thread_pool, bucket_name, s3cmd).unwrap();
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
