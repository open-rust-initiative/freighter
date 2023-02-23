//!
//!
//!
//!
//!
//!

use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use crate::config::ProxyConfig;
use crate::errors::{FreightResult, FreighterError};

use sha2::{Digest, Sha256};

pub trait Download {
    /// download file to a folder with given url and path
    /// return false if connect success but download failed
    fn download_to_folder(&self, msg: &str) -> Result<bool, FreighterError>;
}

/// use reqwest to handle https download requests
#[derive(Default)]
pub struct BlockingReqwest {
    pub opts: DownloadOptions,
}

#[derive(Default, Clone)]
pub struct DownloadOptions {
    pub proxy: ProxyConfig,
    pub url: String,
    pub path: PathBuf,
}

impl Download for BlockingReqwest {
    fn download_to_folder(&self, prefix_msg: &str) -> Result<bool, FreighterError> {
        let DownloadOptions { proxy, url, path } = &self.opts;
        // generate parent folder if not exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).unwrap();
            }
        }
        let client_builder = reqwest::blocking::Client::builder();
        let reqwest_client = if proxy.enable {
            let proxy = reqwest::Proxy::https(proxy.download_proxy.clone()).unwrap();
            client_builder.proxy(proxy).build().unwrap()
        } else {
            client_builder.build().unwrap()
        };
        let mut resp = reqwest_client.get(url).send()?;
        if resp.status() == 200 {
            let mut out = File::create(path).unwrap();
            io::copy(&mut resp, &mut out).unwrap();
            tracing::info!("{} {:?}", prefix_msg, out);
        } else {
            tracing::error!("download failed, Please check your url: {}", url);
            return Ok(false);
        }
        Ok(true)
    }
}

// download remote sha file and then download file for hash check
pub fn download_file_with_sha(
    url: &str,
    file_folder: &Path,
    file_name: &str,
    proxy: &ProxyConfig,
) -> FreightResult {
    let sha_url = format!("{}{}", url, ".sha256");
    let sha_name = format!("{}{}", file_name, ".sha256");
    let sha_path = file_folder.join(sha_name);
    //always update sha256 file
    let opts = &DownloadOptions {
        proxy: proxy.clone(),
        url: sha_url,
        path: sha_path,
    };
    let res = download_and_check_hash(opts, None, true).unwrap();
    if res {
        let content = fs::read_to_string(&opts.path).unwrap();
        let sha256 = &content[..64];
        let opts = &DownloadOptions {
            proxy: proxy.clone(),
            url: url.to_owned(),
            path: file_folder.join(file_name),
        };
        download_and_check_hash(opts, Some(sha256), false).unwrap();
    }
    Ok(())
}

/// download file from remote and calculate it's hash
/// return true if download and success, return false if file already exists
/// -- check_sum: weather need to check hash before download
/// -- is_override: override file if check_sum is none
pub fn download_and_check_hash(
    opts: &DownloadOptions,
    check_sum: Option<&str>,
    is_override: bool,
) -> Result<bool, FreighterError> {
    let br = BlockingReqwest {
        opts: opts.to_owned(),
    };
    let DownloadOptions {
        proxy: _,
        url: _,
        path,
    } = opts;
    if path.is_file() && path.exists() {
        let mut hasher = Sha256::new();
        let mut buffer = BufReader::new(File::open(path)?);
        io::copy(&mut buffer, &mut hasher)?;
        let result = hasher.finalize();
        let hex = format!("{:x}", result);

        //if need to calculate hash
        if let Some(..) = check_sum {
            return if hex == check_sum.unwrap() {
                tracing::info!("###[ALREADY] \t{:?}", buffer.get_ref());
                Ok(false)
            } else {
                tracing::warn!("!!![REMOVE] \t\t {:?} !", buffer.get_ref());
                fs::remove_file(path)?;
                br.download_to_folder("!!![REMOVED DOWNLOAD] \t\t ")
            };
        } else if !is_override {
            tracing::info!(
                "file exist but not pass check_sum, skipping download {}",
                path.display()
            );
            return Ok(false);
        }
    }
    br.download_to_folder("&&&[NEW] \t\t ")
}
