//!
//!
//!
//!
//!
//!

use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
};

use crate::config::ProxyConfig;
use crate::errors::FreighterError;

use sha2::{Digest, Sha256};
use url::form_urlencoded::byte_serialize;
use url::Url;

pub trait Download {
    /// download file to a folder with given url and path
    /// return false if connect success but download failed
    fn download_to_folder(&self, msg: &str) -> Result<bool, FreighterError>;
}

/// use reqwest to handle https download requests
pub struct BlockingReqwest {
    pub opts: DownloadOptions,
}

#[derive(Clone)]
pub struct DownloadOptions {
    pub proxy: ProxyConfig,
    pub url: Url,
    pub path: PathBuf,
}

impl Download for BlockingReqwest {
    fn download_to_folder(&self, prefix_msg: &str) -> Result<bool, FreighterError> {
        let DownloadOptions { proxy, url, path } = &self.opts;

        let client_builder = reqwest::blocking::Client::builder();
        let reqwest_client = if proxy.enable {
            let proxy = reqwest::Proxy::all(proxy.download_proxy.clone()).unwrap();
            client_builder.proxy(proxy).build().unwrap()
        } else {
            client_builder.build().unwrap()
        };
        let mut url = url.clone();
        encode_huaweicloud_url(&mut url);
        let mut resp = reqwest_client.get(url.clone()).send()?;
        if resp.status().is_success() {
            // generate parent folder if not exist
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).unwrap();
                }
            }
            let mut out = BufWriter::new(File::create(path).unwrap());
            io::copy(&mut resp, &mut out).unwrap();
            tracing::info!("{} {:?}", prefix_msg, out.get_ref());
        } else {
            tracing::error!(
                "download failed, Please check your url: {}",
                url.to_string()
            );
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
) -> Result<bool, FreighterError> {
    let sha_url = format!("{}{}", url, ".sha256");
    let sha_name = format!("{}{}", file_name, ".sha256");
    let sha_path = file_folder.join(sha_name);
    //always update sha256 file
    let down_sha = &DownloadOptions {
        proxy: proxy.clone(),
        url: Url::parse(&sha_url).unwrap(),
        path: sha_path,
    };
    let res = download_and_check_hash(down_sha, None, true).unwrap();
    if res {
        let content = fs::read_to_string(&down_sha.path).unwrap();
        let sha256 = &content[..64];
        let down_file = &DownloadOptions {
            proxy: proxy.clone(),
            url: Url::parse(url).unwrap(),
            path: file_folder.join(file_name),
        };
        download_and_check_hash(down_file, Some(sha256), false)
    } else {
        Ok(false)
    }
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
    let path = &opts.path;
    if path.is_file() && path.exists() {
        let mut hasher = Sha256::new();
        let mut file = File::open(path)?;
        io::copy(&mut file, &mut hasher)?;
        let result = hasher.finalize();
        let hex = format!("{:x}", result);

        //if need to calculate hash
        if let Some(..) = check_sum {
            return if hex == check_sum.unwrap() {
                tracing::info!("###[ALREADY] \t{:?}", file);
                Ok(false)
            } else {
                tracing::warn!("!!![REMOVE] \t\t {:?} !", file);
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

pub fn encode_huaweicloud_url(url: &mut Url) {
    if let Some(domain) = url.domain() {
        if domain.contains("myhuaweicloud.com") && url.path().starts_with("/crates") {
            let mut path = PathBuf::from(url.path());
            let encode_path: String =
                byte_serialize(path.file_name().unwrap().to_str().unwrap().as_bytes()).collect();
            path.pop();
            path.push(&encode_path);
            url.set_path(path.to_str().unwrap());
        }
    }
}

#[cfg(test)]
mod tests {

    use reqwest::Url;

    use crate::download;

    #[test]
    fn test_huaweicloud_url_serial() {
        let mut url = Url::parse("https://rust-proxy.obs.cn-east-3.myhuaweicloud.com/crates/google-coordinate1/google-coordinate1-0.1.1+20141215.crate").unwrap();
        download::encode_huaweicloud_url(&mut url);
        assert_eq!(url.to_string(), "https://rust-proxy.obs.cn-east-3.myhuaweicloud.com/crates/google-coordinate1/google-coordinate1-0.1.1%2B20141215.crate");

        // Skip routes that don't start with /crates
        let mut url = Url::parse("https://rust-proxy.obs.cn-east-3.myhuaweicloud.com/dist/2023-06-05/google-coordinate1-0.1.1+20141215.crate").unwrap();
        download::encode_huaweicloud_url(&mut url);
        assert_eq!(url.to_string(), "https://rust-proxy.obs.cn-east-3.myhuaweicloud.com/dist/2023-06-05/google-coordinate1-0.1.1+20141215.crate");
    }
}
