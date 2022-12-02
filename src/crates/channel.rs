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
};

use chrono::{Duration, NaiveDate, Utc};
use log::{info, error};
use serde::Deserialize;
use threadpool::ThreadPool;
use walkdir::WalkDir;

use crate::{
    config::RustUpConfig,
    download::{download_file, download_file_with_sha},
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

    /// Whether to clean historical versions.
    pub clean: bool,
    /// only sync that version
    pub version: Option<String>,

    pub dist_path: PathBuf,

    pub bucket_name: String,
}

/// entrance function
pub fn sync_rust_toolchain(opts: &ChannelOptions) -> FreightResult {
    let config = &opts.config;
    if let Some(version) = &opts.version {
        // step1.1 : sync input channel version
        sync_channel(opts, version)?;
    } else {
        // step1.2: sync latest stable,beta and nightly channel
        sync_channel(opts, "stable")?;
        sync_channel(opts, "beta")?;
        sync_channel(opts, "nightly")?;
        // step1.3: sync specified channel version by config file
        config.sync_stable_versions.iter().for_each(|channel| {
            sync_channel(opts, channel).unwrap();
        });
    }
    // step2: clean historical channel files if needed
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
    let file_folder;
    if let Some(date) = channel.strip_prefix("nightly-") {
        channel_name = String::from("channel-rust-nightly.toml");
        channel_url = format!("{}/dist/{}/{}", opts.config.domain, date, channel_name);
        file_folder = opts.dist_path.to_owned().join(date);
    } else {
        channel_name = format!("channel-rust-{}.toml", channel);
        channel_url = format!("{}/dist/{}", opts.config.domain, channel_name);
        file_folder = opts.dist_path.to_owned();
    }
    match download_file_with_sha(&channel_url, &file_folder, &channel_name) {
        Ok(_) => {
            let pool = ThreadPool::new(opts.config.download_threads);
            // parse_channel_file and download;
            let download_list = parse_channel_file(&file_folder.join(channel_name)).unwrap();
            download_list.into_iter().for_each(|(url, hash)| {
                // example: https://static.rust-lang.org/dist/2022-11-03/rust-1.65.0-i686-pc-windows-gnu.msi
                // remove url prefix "https://static.rust-lang.org/dist"
                let path: PathBuf = std::iter::once(opts.dist_path.to_owned())
                    .chain(
                        url.split('/').map(PathBuf::from).collect::<Vec<PathBuf>>()[4..].to_owned(),
                    )
                    .collect();
                pool.execute(move || {
                    download_file(&url, &path, Some(&hash), false).unwrap();
                });
            });
            pool.join();
        }
        Err(_err) => {
            info!("skipping download channel:{}", channel);
        }
    }
    Ok(())
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
                        info!("!!![REMOVE] \t\t {:?} !", entry.path());
                    }
                });
            // remove whole directory when it's empty
            if entry.path().read_dir().unwrap().next().is_none() {
                fs::remove_dir_all(entry.path()).unwrap();
                info!("!!![REMOVE] \t\t {:?} !", entry.path());
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
                error!(
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

