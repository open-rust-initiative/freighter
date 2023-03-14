//! **crates** subcommand focus on the sync crates index and crate files from upstream. The core
//! function implemented in the `src/crates/index`.
//!
//! **crates** subcommand provide major functions include:
//!
//!   Arguments:
//!   - __domain__: you can choose your own upstream by adding this argument in command,
//!         this param can be changed in the configuration file or pass it here
//!   - __download-threads__: specify the download threads to parallel download,
//!         this param can be changed in the configuration file or pass it here
//!   - __no-progressbar__: Whether to hide progress bar when start downloading
//!
//! # pull subcommand
//!
//!   sync crates index from upstream to local:
//!
//!   - The crates index is a git repository, and **cargo** clone and update from [GitHub](https://github.com/rust-lang/crates.io-index).
//!     - The clone use `bare` mode, more details in the [cargo guide](https://github.com/rust-lang/cargo/blob/6b6b0b486d73c03ed952591d880debec1d47c534/src/doc/src/guide/cargo-home.md#directories)
//!   
//! # download subcommand
//!   sync crate file from upstream to local:
//!     
//!   - The crate file of upstream location detail from [crates.io-index](https://github.com/rust-lang/crates.io-index/blob/master/.github/workflows/update-dl-url.yml)
//!      ```YAML
//!      env:
//!         URL_api: "https://crates.io/api/v1/crates"
//!         URL_cdn: "https://static.crates.io/crates/sync{crate}/{crate}-{version}.crate"
//!         URL_s3_primary: "https://crates-io.s3-us-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
//!         URL_s3_fallback: "https://crates-io-fallback.s3-eu-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
//!      ```
//!
//!   Arguments:
//!   - __init__: Whether to download all the crates files for initialization.
//!   - __upload__: Whether to upload single file to s3 after download success.
//!   - __bucket__: set the s3 bucket you want to upload files to, you must provide this param befor uplaod.
//!   - __delete-after-upload__: This optional parameter will be used to delete files after upload.
//!
//! # upload subcommand
//!
//!   - Sync crate file to Object Storage Service compatible with [AWS S3](https://aws.amazon.com/s3/)
//!     - Digitalocean Spaces
//!     - Huawei Cloud OBS
//!     - Alibaba Cloud OSS
//!     - Tencent Cloud COS
//!     - AWS S3
//!     - minio
//!     - Ceph
//!   Arguments:
//!   - __bucket__: set the s3 bucket you want to upload files to, you must provide this param before upload.
//!  

use clap::{arg, ArgMatches};

use crate::commands::command_prelude::*;
use crate::config::Config;
use crate::errors::FreightResult;
use crate::handler::crates_file::{download, upload_to_s3, CratesOptions};
use crate::handler::index::{pull, CrateIndex};
use crate::handler::DownloadMode;

/// The __crates__ subcommand
///

pub fn cli() -> clap::Command {
    clap::Command::new("crates")
        .arg(flag("no-progressbar", "Hide progressbar when start sync"))
        .arg(arg!(-t --"download-threads" <VALUE> "specify the download threads to parallel download, 
        this param can be changed in the configuration file or pass it here")
            .value_parser(value_parser!(usize))
        )
        .arg(arg!(-d --"domain" <VALUE> "specify the source you want to sync from, 
        this param can be changed in the configuration file or pass it here"))
        .subcommand(subcommand("pull"))
        .subcommand(subcommand("upload")
        .arg(arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files").required(true))
        .arg(arg!(--"name" <VALUE> "only upload specify crates").required(true))
        )
        .subcommand(subcommand("download")
            .arg(flag("init", "Start init download of crates file, this will traverse all index for full download"))
            .arg(flag("fix", "Hanlde the crates file that download failed, this opetion will traverse error log"))
            .arg(flag("upload", "upload every crate file after download"))
            .arg(arg!(-b --"bucket" <VALUE> "set the s3 bucket name you want to upload files"))
            .arg(flag("delete-after-upload", "this will delete file after upload"))
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Sync the crates from the upstream(crates.io) to the local registry")
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

       freighter -c /mnt/volume_fra1_01 crates pull

2. Download all crates file and unload:

       freighter crates download --init --upload --bucket crates

3. Download crates file with multi-thread to specify directory:

       freighter -c /mnt/volume_fra1_01 crates -t 32 download --init

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

    crate::cli::init_log(&config.log, work_dir.to_path_buf(), "crates").unwrap();

    let opts = &mut CratesOptions {
        config: config.crates.to_owned(),
        proxy: config.proxy.to_owned(),
        index: CrateIndex::new(&config.crates.index_domain, work_dir.to_owned()),
        no_progressbar: args.get_flag("no-progressbar"),
        crates_path: work_dir.join("crates"),
        log_path: work_dir.join("log"),
        ..Default::default()
    };
    let domain = args.get_one::<String>("domain").cloned();

    match args.get_one::<usize>("download-threads").cloned() {
        Some(download_threads) => opts.config.download_threads = download_threads,
        None => tracing::info!("use default thread count: {}", opts.config.download_threads),
    };

    tracing::info!("CratesOptions info : {:#?}", opts);

    match args.subcommand() {
        Some(("pull", _args)) => {
            if let Some(source) = domain {
                config.crates.index_domain = source;
            }
            pull(opts)?
        }
        Some(("download", args)) => {
            opts.upload = args.get_flag("upload");
            opts.download_mode = DownloadMode::new(args.get_flag("init"), args.get_flag("fix"));
            opts.delete_after_upload = args.get_flag("delete-after-upload");
            let bucket_name = args.get_one::<String>("bucket").cloned();
            if opts.upload {
                if bucket_name.is_none() {
                    unreachable!("can not upload with empty bucket name")
                } else {
                    opts.bucket_name = bucket_name.unwrap();
                }
            }
            if let Some(source) = domain {
                config.crates.domain = source;
            }
            download(opts)?
        }
        Some(("upload", args)) => {
            opts.bucket_name = args.get_one::<String>("bucket").cloned().unwrap();
            opts.crates_name = args.get_one::<String>("name").cloned();
            upload_to_s3(opts)?
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
