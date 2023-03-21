//!
//!
//!
//!
//!
//!

use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use threadpool::ThreadPool;
use walkdir::WalkDir;

use crate::{
    cloud::{s3::S3cmd, CloudStorage},
    config::{ProxyConfig, RustUpConfig},
    download::{download_and_check_hash, download_file_with_sha, DownloadOptions},
    errors::{FreightResult, FreighterError},
};

#[derive(Debug, Deserialize)]
pub struct Channel {
    #[serde(alias = "manifest-version")]
    pub manifest_version: String,
    pub date: String,
    pub pkg: HashMap<String, Pkg>,
}

#[derive(Debug, Deserialize)]
pub struct Pkg {
    pub version: String,
    pub target: HashMap<String, Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xz_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xz_hash: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct ChannelOptions {
    pub config: RustUpConfig,

    pub proxy: ProxyConfig,

    /// Whether to clean historical versions.
    pub clean: bool,
    /// only sync that version
    pub version: Option<String>,

    pub dist_path: PathBuf,

    pub bucket: Option<String>,

    pub upload: bool,

    pub delete_after_upload: bool,

    pub sync_history: bool,

    pub init: bool,
}

/// entrance function
pub fn sync_rust_toolchain(opts: &ChannelOptions) -> FreightResult {
    let config = &opts.config;
    if let Some(version) = &opts.version {
        // step 1 : sync specified channel version
        sync_channel(opts, version)?;
    } else if opts.sync_history {
        // step 2: sync historical nightly and beta versions
        if let Some(date) = config.history_version_start_date.clone() {
            let start_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap();
            tracing::info!(
                "step 2: sync historical nightly and beta versions from {}",
                start_date
            );
            let today = Utc::now().date_naive();
            if today >= start_date {
                let duration_days = (today - start_date).num_days().try_into().unwrap();
                for (_, day) in start_date.iter_days().take(duration_days).enumerate() {
                    sync_channel(opts, &format!("beta-{}", day))?;
                    sync_channel(opts, &format!("nightly-{}", day))?;
                }
            } else {
                tracing::error!("start date {} is after today {}", start_date, today);
            }
        }
    } else {
        // step 3.1: sync latest stable, beta and nightly channel
        tracing::info!("step 3.1: sync latest stable, beta and nightly channel");
        sync_channel(opts, "stable")?;
        sync_channel(opts, "beta")?;
        sync_channel(opts, "nightly")?;
        if opts.init {
            // step 3.2: sync specified channel version by config file
            tracing::info!("step 3.2:(optional) sync specified channel version by config file");
            config.sync_stable_versions.iter().for_each(|channel| {
                sync_channel(opts, channel).unwrap();
            });
        }
    }
    // step 3: clean local historical channel files if needed
    if opts.clean {
        let channels = [
            ("beta", config.sync_beta_days),
            ("nightly", config.sync_nightly_days),
        ];
        for channel in channels {
            clean_historical_version(&opts.dist_path, channel).unwrap();
        }
    }
    Ok(())
}

// sync the latest toolchain by given a channel name(stable, beta, nightly) or history version by version number
pub fn sync_channel(opts: &ChannelOptions, channel: &str) -> FreightResult {
    let channel_name;
    let channel_url;
    let channel_folder;
    tracing::info!("starting download channel: {}", channel);
    if let Some(date) = channel.strip_prefix("nightly-") {
        channel_name = String::from("channel-rust-nightly.toml");
        channel_url = format!("{}/dist/{}/{}", opts.config.domain, date, channel_name);
        channel_folder = opts.dist_path.to_owned().join(date);
    } else if let Some(date) = channel.strip_prefix("beta-") {
        channel_name = String::from("channel-rust-beta.toml");
        channel_url = format!("{}/dist/{}/{}", opts.config.domain, date, channel_name);
        channel_folder = opts.dist_path.to_owned().join(date);
    } else {
        channel_name = format!("channel-rust-{}.toml", channel);
        channel_url = format!("{}/dist/{}", opts.config.domain, channel_name);
        channel_folder = opts.dist_path.to_owned();
    }
    match download_file_with_sha(&channel_url, &channel_folder, &channel_name, &opts.proxy) {
        Ok(res) => {
            let channel_toml = &channel_folder.join(channel_name);
            if !res && !channel_toml.exists() {
                tracing::error!("skipping channel: {}", channel);
                return Ok(());
            }
            let pool = ThreadPool::new(opts.config.download_threads);
            // parse_channel_file and download;
            let download_list = parse_channel_file(channel_toml).unwrap();
            let s3cmd = Arc::new(S3cmd::default());
            download_list.into_iter().for_each(|(url, hash)| {
                // example: https://static.rust-lang.org/dist/2022-11-03/rust-1.65.0-i686-pc-windows-gnu.msi
                // these code was used to remove url prefix "https://static.rust-lang.org/dist"
                // and get "2022-11-03/rust-1.65.0-i686-pc-windows-gnu.msi"
                let path: PathBuf = std::iter::once(opts.dist_path.to_owned())
                    .chain(
                        url.split('/').map(PathBuf::from).collect::<Vec<PathBuf>>()[4..].to_owned(),
                    )
                    .collect();
                let (upload, dist_path, bucket, delete_after_upload) = (
                    opts.upload,
                    opts.dist_path.to_owned(),
                    opts.bucket.to_owned(),
                    opts.delete_after_upload,
                );
                let s3cmd = s3cmd.clone();
                let proxy = opts.proxy.clone();
                pool.execute(move || {
                    let down_opts = &DownloadOptions { proxy, url, path };
                    let path = &down_opts.path;
                    let downloaded =
                        download_and_check_hash(down_opts, Some(&hash), false).unwrap();
                    if downloaded && upload {
                        let s3_path = format!(
                            "dist{}",
                            path.to_str()
                                .unwrap()
                                .replace(dist_path.to_str().unwrap(), "")
                        );
                        let uploaded = s3cmd.upload_file(path, &s3_path, &bucket.unwrap());
                        if uploaded.is_ok() && delete_after_upload {
                            fs::remove_file(path).unwrap();
                        }
                    }
                });
            });
            pool.join();
            replace_toml_and_sha(opts, s3cmd, channel_toml);
        }
        Err(_err) => {
            tracing::info!("skipping download channel:{}", channel);
        }
    }
    Ok(())
}

// upload toml file and sha256 after all files handle success
pub fn replace_toml_and_sha(opts: &ChannelOptions, s3cmd: Arc<S3cmd>, channel_toml: &Path) {
    let shafile = channel_toml.with_extension("toml.sha256");
    let files: Vec<&Path> = vec![channel_toml, &shafile];
    if opts.upload {
        for file in files {
            let s3_path = format!(
                "dist{}",
                file.to_str()
                    .unwrap()
                    .replace(opts.dist_path.to_str().unwrap(), "")
            );
            s3cmd
                .upload_file(file, &s3_path, &opts.bucket.clone().unwrap())
                .unwrap();
        }
    }
}
// parse channel file to get download url and hash
pub fn parse_channel_file(path: &Path) -> Result<Vec<(String, String)>, FreighterError> {
    let content = fs::read_to_string(path).unwrap();
    let channel: Channel = toml::from_str(&content).unwrap();
    let res: Vec<(String, String)> = channel
        .pkg
        .into_iter()
        .flat_map(|(_, pkg)| {
            pkg.target
                .into_iter()
                .flat_map(|(_, target)| -> Vec<(String, String)> {
                    let mut result: Vec<(String, String)> = Vec::new();
                    if target.xz_url.is_some() && target.xz_hash.is_some() {
                        result.push((target.xz_url.unwrap(), target.xz_hash.unwrap()));
                    }
                    if target.url.is_some() && target.hash.is_some() {
                        let url = target.url.unwrap();
                        let hash = target.hash.unwrap();
                        if !url.is_empty() && !hash.is_empty() {
                            result.push((url, hash));
                        }
                    }
                    result
                })
        })
        .collect();
    Ok(res)
}

pub fn clean_historical_version(dist_path: &PathBuf, channels: (&str, i64)) -> FreightResult {
    let (channel, sync_days) = channels;
    // filter dir less than sync_nightly_days ago
    fs::read_dir(dist_path)
        .unwrap()
        .filter_map(|v| v.ok())
        .filter(|entry| compare_date(entry, sync_days))
        .for_each(|entry| {
            WalkDir::new(entry.path())
                .into_iter()
                .filter_map(|f| f.ok())
                .for_each(|entry| {
                    let file_name = entry.file_name().to_str().unwrap();
                    if file_name.contains(channel) {
                        fs::remove_file(entry.path()).unwrap();
                        tracing::info!("!!![REMOVE] \t\t {:?} !", entry.path());
                    }
                });
            // remove whole directory when it's empty
            if entry.path().read_dir().unwrap().next().is_none() {
                fs::remove_dir_all(entry.path()).unwrap();
                tracing::info!("!!![REMOVE] \t\t {:?} !", entry.path());
            }
        });

    Ok(())
}

pub fn compare_date(entry: &DirEntry, sync_days: i64) -> bool {
    if entry.file_type().unwrap().is_dir() {
        let date = match NaiveDate::parse_from_str(entry.file_name().to_str().unwrap(), "%Y-%m-%d")
        {
            Ok(date) => date,
            Err(_) => {
                tracing::error!(
                    "can't parse dir :{} and skipping... ",
                    entry.path().display()
                );
                return false;
            }
        };
        Utc::now().date_naive() - date > Duration::days(sync_days)
    } else {
        false
    }
}
