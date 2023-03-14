//!
//!
//!
//!
//!
//!

use std::io::Write;

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::cloud::s3::S3cmd;
use crate::cloud::{self, CloudStorage};
use crate::config::{CratesConfig, ProxyConfig};
use crate::download::{download_and_check_hash, DownloadOptions};
use crate::errors::FreightResult;
use crate::handler::index;

use super::index::CrateIndex;
use super::DownloadMode;

/// CratesOptions preserve the sync subcommand config
#[derive(Clone, Default, Debug)]
pub struct CratesOptions {
    pub config: CratesConfig,

    pub proxy: ProxyConfig,

    pub index: CrateIndex,

    /// Whether to hide progressbar when start sync.
    pub no_progressbar: bool,

    /// start traverse all directories
    pub download_mode: DownloadMode,

    pub upload: bool,

    pub crates_path: PathBuf,

    pub crates_name: Option<String>,

    pub log_path: PathBuf,

    pub bucket_name: String,

    pub delete_after_upload: bool,
}

/// Crate preserve the crates info parse from registry json file
#[derive(Serialize, Deserialize, Debug)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<BTreeMap<String, Vec<String>>>,
    pub yanked: Option<bool>,
    #[serde(default)]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u32>,
}

/// Dependencies maintain relationships between crate
///
///
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

/// DependencyKind represents which stage the current dependency is
///
///
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

/// full download and Incremental download from registry
pub fn download(opts: &CratesOptions) -> FreightResult {
    match opts.download_mode {
        DownloadMode::Init => full_downloads(opts).unwrap(),
        DownloadMode::Fix => fix_download(opts).unwrap(),
        DownloadMode::Increment => incremental_download(opts).unwrap(),
    }
    Ok(())
}

/// <https://github.com/rust-lang/crates.io-index/blob/master/.github/workflows/update-dl-url.yml>
///
/// ```YAML
///env:
///   URL_api: "https://crates.io/api/v1/crates"
///   URL_cdn: "https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
///   URL_s3_primary: "https://crates-io.s3-us-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
///   URL_s3_fallback: "https://crates-io-fallback.s3-eu-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
/// ```
pub fn full_downloads(opts: &CratesOptions) -> FreightResult {
    let pool = ThreadPool::new(opts.config.download_threads);
    let err_record = open_file_with_mutex(&opts.log_path);

    WalkDir::new(&opts.index.path)
        .into_iter()
        .filter_entry(is_not_hidden)
        .filter_map(|v| v.ok())
        .for_each(|x| {
            if x.file_type().is_file() && x.path().extension().unwrap_or_default() != "json" {
                parse_index_and_download(&x.path().to_path_buf(), opts, &pool, &err_record)
                    .unwrap();
            }
        });
    pool.join();
    tracing::info!("sync ends with {} task failed", pool.panic_count());
    Ok(())
}

pub fn incremental_download(opts: &CratesOptions) -> FreightResult {
    let it = WalkDir::new(&opts.log_path)
        .into_iter()
        .filter_entry(|e| {
            e.file_name()
                .to_str()
                .unwrap()
                .contains(&Utc::now().date_naive().to_string())
                || e.file_type().is_dir()
        })
        .filter_map(|v| v.ok());
    let mut input = match it.last() {
        Some(dir) => {
            if dir.file_type().is_file() {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(dir.path())
                    .unwrap()
            } else {
                panic!("Cannot get record file, run freighter crates pull before download")
            }
        }
        None => panic!("Did you forget to run freighter crates pull before download?"),
    };
    let buffered = BufReader::new(&mut input);
    tracing::info!("crates.io-index modified:");
    let err_record = open_file_with_mutex(&opts.log_path);
    // get last line of record file
    let mut lines: Vec<String> = buffered.lines().map(|line| line.unwrap()).collect();
    lines.reverse();
    if let Some(line) = lines.first() {
        let vec: Vec<&str> = line.split(',').collect();
        tracing::info!("{:?}", line);
        index::git2_diff(opts, vec[0], vec[1], err_record).unwrap();
    }
    Ok(())
}

/// fix the previous error download crates
pub fn fix_download(opts: &CratesOptions) -> FreightResult {
    let pool = ThreadPool::new(opts.config.download_threads);

    let file_name = &opts.log_path.join("error-crates.log");
    let err_record = OpenOptions::new().read(true).open(file_name).unwrap();
    let buffered = BufReader::new(err_record);
    let mut visited: HashSet<String> = HashSet::new();

    let err_record_with_mutex = open_file_with_mutex(&opts.log_path);

    for line in buffered.lines() {
        let unwrap_line = line.unwrap();
        let vec: Vec<&str> = unwrap_line.split(", ").collect();
        let crates_name_version = vec[0];
        let mut name_split: Vec<&str> = crates_name_version.split('-').collect();
        name_split.pop();
        let crates_name = name_split.into_iter().collect::<String>();
        if !visited.contains(&crates_name) {
            if crates_name.len() >= 4 {
                let suffix = format!(
                    "{}/{}/{}",
                    &crates_name[0..2],
                    &crates_name[2..4],
                    crates_name
                );
                let index_path = opts.index.path.join(suffix);
                parse_index_and_download(&index_path, opts, &pool, &err_record_with_mutex).unwrap();
                visited.insert(crates_name.to_owned());
                tracing::info!("handle success: {:?}", &crates_name_version);
            }
            // skip the crates which name less than 4 bytes
        } else {
            // skip visited
            tracing::info!(
                "skip different verion of same crates: {}",
                crates_name_version
            );
        }
    }
    pool.join();
    tracing::info!("sync ends with {} task failed", pool.panic_count());
    if pool.panic_count() == 0 {
        fs::remove_file(file_name).unwrap();
    }
    Ok(())
}

pub fn upload_to_s3(opts: &CratesOptions) -> FreightResult {
    let s3cmd = S3cmd::default();
    if opts.crates_name.is_none() {
        cloud::upload_with_pool(
            opts.config.download_threads,
            opts.crates_path.clone(),
            opts.bucket_name.clone(),
            s3cmd,
        )
        .unwrap();
    } else {
        cloud::upload_single_dir(
            opts.crates_path.clone(),
            opts.crates_name.clone().unwrap(),
            opts.bucket_name.clone(),
            s3cmd,
        )
    }
    Ok(())
}

/// open error record file with Mutex
pub fn open_file_with_mutex(log_path: &Path) -> Arc<Mutex<File>> {
    let file_name = log_path.join("error-crates.log");
    let err_record = match OpenOptions::new().write(true).append(true).open(&file_name) {
        Ok(f) => Arc::new(Mutex::new(f)),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Arc::new(Mutex::new(File::create(&file_name).unwrap())),
            other_error => panic!("something wrong: {}", other_error),
        },
    };
    err_record
}

/// Check whether the directory is hidden
pub fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with('.'))
        .unwrap_or(false)
}

pub fn parse_index_and_download(
    index_path: &PathBuf,
    opts: &CratesOptions,
    pool: &ThreadPool,
    err_record: &Arc<Mutex<File>>,
) -> FreightResult {
    match File::open(index_path) {
        Ok(f) => {
            let buffered = BufReader::new(f);

            for line in buffered.lines() {
                let line = line.unwrap();
                let c: Crate = serde_json::from_str(&line).unwrap();
                let err_record = Arc::clone(err_record);
                let opts = opts.clone();

                let url = format!(
                    "{}/{}/{}-{}.crate",
                    opts.config.domain, &c.name, &c.name, &c.vers
                );

                let file = opts
                    .crates_path
                    .join(&c.name)
                    .join(format!("{}-{}.crate", &c.name, &c.vers));

                pool.execute(move || {
                    download_crates_with_log(file, &opts, url, c, err_record).unwrap();
                });
            }
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                tracing::warn!(
                    "This file might have been removed from crates.io:{}",
                    &index_path.display()
                );
            }
            other_error => panic!("something wrong while open the index file: {}", other_error),
        },
    };
    Ok(())
}

pub fn download_crates_with_log(
    path: PathBuf,
    opts: &CratesOptions,
    url: String,
    c: Crate,
    err_record: Arc<Mutex<File>>,
) -> FreightResult {
    let down_opts = &DownloadOptions {
        proxy: opts.proxy.clone(),
        url,
        path,
    };

    match download_and_check_hash(down_opts, Some(&c.cksum), false) {
        Ok(download_succ) => {
            let path = &down_opts.path;
            if download_succ && opts.upload {
                let s3 = S3cmd::default();
                let s3_path = format!(
                    "crates{}",
                    path.to_str()
                        .unwrap()
                        .replace(opts.crates_path.to_str().unwrap(), "")
                );
                tracing::info!("s3_path: {}, {}", s3_path, opts.delete_after_upload);
                let uploded = s3.upload_file(path, &s3_path, &opts.bucket_name);
                if uploded.is_ok() && opts.delete_after_upload {
                    fs::remove_file(path).unwrap();
                }
            }
            Ok(())
        }
        Err(err) => {
            let mut err_file = err_record.lock().unwrap();
            writeln!(
                err_file,
                "{}-{}.crate, {}",
                &c.name,
                &c.vers,
                Utc::now().timestamp()
            )
            .unwrap();
            tracing::error!("{:?}", err);
            Err(err)
        }
    }
}
