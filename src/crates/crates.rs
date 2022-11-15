use chrono::Utc;

use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

use crate::config::CratesConfig;
use crate::crates::index;
use crate::download::{download_crates_with_log, sync_folder};
use crate::errors::FreightResult;

use super::index::CrateIndex;

/// SyncOptions preserve the sync subcommand config
#[derive(Clone, Default, Debug)]
pub struct SyncOptions {
    pub config: CratesConfig,

    pub index: CrateIndex,

    /// Whether to hide progressbar when start sync.
    pub no_progressbar: bool,
    /// start traverse all directories
    pub init_download: bool,

    pub upload: bool,

    pub work_dir: PathBuf,

    pub crates_path: PathBuf,

    pub log_path: PathBuf,
}



/// Crate preserve the crates info parse from registry json file
///
///
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
pub fn download(opts: &mut SyncOptions) -> FreightResult {
    if opts.init_download {
        full_downloads(opts).unwrap();
    } else {
        let it = WalkDir::new(&opts.log_path)
            .into_iter()
            .filter_entry(|e| {
                e.file_name()
                    .to_str()
                    .unwrap()
                    .contains(&Utc::now().date().to_string())
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
                    panic!("Cannot get record file, run freighter sync pull before download")
                }
            }
            None => panic!("Did you forget to run freighter sync pull before download?"),
        };
        let buffered = BufReader::new(&mut input);
        println!("crates.io-index modified:");
        let err_record = open_file_with_mutex(&opts.log_path);
        // get last line of record file
        let mut lines: Vec<String> = buffered.lines().map(|line| line.unwrap()).collect();
        lines.reverse();
        if let Some(line) = lines.first() {
            let vec: Vec<&str> = line.split(',').collect();
            println!("{:?}", line);
            index::git2_diff(opts, vec[0], vec[1], err_record).unwrap();
        }
    }

    Ok(())
}

/// https://github.com/rust-lang/crates.io-index/blob/master/.github/workflows/update-dl-url.yml
///
/// ```YAML
///env:
///   URL_api: "https://crates.io/api/v1/crates"
///   URL_cdn: "https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
///   URL_s3_primary: "https://crates-io.s3-us-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
///   URL_s3_fallback: "https://crates-io-fallback.s3-eu-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
/// ```
pub fn full_downloads(opts: &SyncOptions) -> FreightResult {
    let pool = ThreadPool::new(opts.config.download_threads);
    let err_record = open_file_with_mutex(&opts.log_path);

    WalkDir::new(&opts.index.path)
        .into_iter()
        .filter_entry(|e| is_not_hidden(e))
        .filter_map(|v| v.ok())
        .for_each(|x| {
            if x.file_type().is_file() && x.path().extension().unwrap_or_default() != "json" {
                parse_index_and_download(x.path().to_path_buf(), opts, &pool, &err_record).unwrap();
            }
        });
    pool.join();
    println!("sync ends with {} task failed", pool.panic_count());
    Ok(())
}

pub fn upload_to_s3(opts: &SyncOptions, bucket_name: &str) -> FreightResult {
    // let sync_paths = [&opts.crates_path, &opts.rustup_path, &opts.dist_path];
    // for path in sync_paths {
    sync_folder(opts.crates_path.to_str().unwrap(), bucket_name).unwrap();
    // }
    Ok(())
}

/// open error record file with Mutex
pub fn open_file_with_mutex(log_path: &Path) -> Arc<Mutex<File>> {
    let file_name = log_path.join("error-crates.cache");
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
    index_path: PathBuf,
    opts: &SyncOptions,
    pool: &ThreadPool,
    err_record: &Arc<Mutex<File>>,
) -> FreightResult {
    match File::open(&index_path) {
        Ok(f) => {
            let buffered = BufReader::new(f);

            for line in buffered.lines() {
                let line = line.unwrap();
                let c: Crate = serde_json::from_str(&line).unwrap();
                let err_record = Arc::clone(err_record);
                let opts = opts.clone();
                pool.execute(move || {
                    download_crates_with_log(
                        opts.crates_path,
                        opts.upload,
                        opts.config.domain,
                        c,
                        err_record,
                    );
                });
            }
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                println!(
                    "This file might have been removed from crates.io:{}",
                    &index_path.display()
                );
            }
            other_error => panic!(
                "something wrong while open the crates file: {}",
                other_error
            ),
        },
    };
    Ok(())
}
