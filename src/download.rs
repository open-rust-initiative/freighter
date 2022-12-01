//!
//!
//!
//!
//!
//!

use std::{
    fs::{self, File},
    io,
    path::Path,
};

use log::{error, info, warn};
use sha2::{Digest, Sha256};

use crate::errors::{FreightResult, FreighterError};

pub trait Download {
    /// download file to a folder with given url and path
    /// return false if connect success but download failed
    fn download_to_folder(&self, url: &str, path: &Path, msg: &str)
        -> Result<bool, FreighterError>;
}

#[derive(Default)]
pub struct BlockingReqwest {}

#[derive(Default)]
pub struct Reqwest {}

impl Download for BlockingReqwest {
    fn download_to_folder(
        &self,
        url: &str,
        path: &Path,
        msg: &str,
    ) -> Result<bool, FreighterError> {
        // generate parent folder if unexist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).unwrap();
            }
        }
        let mut resp = reqwest::blocking::get(url)?;
        if resp.status() == 200 {
            let mut out = File::create(path).unwrap();
            io::copy(&mut resp, &mut out).unwrap();
            info!("{} {:?}", msg, out);
        } else {
            error!("download failed, Please check your url: {}", url);
            return Ok(false);
        }
        Ok(true)
    }
}

// download remote sha file and then download file for hash check
pub fn download_file_with_sha(url: &str, file_folder: &Path, file_name: &str) -> FreightResult {
    let sha_url = format!("{}{}", url, ".sha256");
    let sha_name = format!("{}{}", file_name, ".sha256");
    let sha_path = file_folder.join(sha_name);
    //always update sha256 file
    let res = download_file(&sha_url, &sha_path, None, true).unwrap();
    if res {
        let content = fs::read_to_string(&sha_path).unwrap();
        let sha256 = &content[..64];
        download_file(url, &file_folder.join(file_name), Some(sha256), false).unwrap();
    }
    Ok(())
}

/// download file from remote and calculate it's hash
/// return true if download and success, return flase if file already exists
pub fn download_file(
    url: &str,
    path: &Path,
    check_sum: Option<&str>,
    is_override: bool,
) -> Result<bool, FreighterError> {
    let br = BlockingReqwest::default();
    if path.is_file() && path.exists() {
        let mut hasher = Sha256::new();
        let mut f = File::open(path)?;
        io::copy(&mut f, &mut hasher)?;
        let result = hasher.finalize();
        let hex = format!("{:x}", result);

        //if need to calculate hash
        if let Some(..) = check_sum {
            if hex == check_sum.unwrap() {
                info!("###[ALREADY] \t{:?}", f);
                return Ok(false);
            } else {
                warn!("!!![REMOVE] \t\t {:?} !", f);
                fs::remove_file(path)?;
                return br.download_to_folder(url, path, "!!![REMOVED DOWNLOAD] \t\t ");
            }
        } else if !is_override {
            info!(
                "file exist but not pass check_sum, skiping download {}",
                path.display()
            );
            return Ok(false);
        }
    }
    br.download_to_folder(url, path, "&&&[NEW] \t\t ")
}
