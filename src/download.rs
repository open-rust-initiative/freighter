//!
//!
//!
//!
//!
//!

use std::io::Write;
use std::path::PathBuf;
use std::{
    fs::{self, File},
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use log::{error, info, warn};
use sha2::{Digest, Sha256};

use crate::crates::crates_file::Crate;
use crate::errors::{FreightResult, FreighterError};

// download remote sha file and then download file for hash check
pub fn download_file_with_sha(url: &str, file_folder: &Path, file_name: &str) -> FreightResult {
    let sha_url = format!("{}{}", url, ".sha256");
    let sha_name = format!("{}{}", file_name, ".sha256");
    let sha_path = file_folder.join(&sha_name);
    //always update sha256 file
    let res = download_file(&sha_url, &sha_path, None, true).unwrap();
    if res {
        let content = fs::read_to_string(&sha_path).unwrap();
        let sha256 = &content[..64];
        download_file(url, &file_folder.join(file_name), Some(sha256), false).unwrap();
    }
    Ok(())
}

pub fn download_crates_with_log(
    path: PathBuf,
    upload: bool,
    url: String,
    c: Crate,
    err_record: Arc<Mutex<File>>,
) {
    let url = format!("{}/{}/{}-{}.crate", url, &c.name, &c.name, &c.vers);
    let folder = path.join(&c.name);
    let file = folder.join(format!("{}-{}.crate", &c.name, &c.vers));
    match download_file(&url, &file, Some(&c.cksum), false) {
        Ok(download_succ) => {
            if download_succ && upload {
                upload_file(
                    file.to_str().unwrap(),
                    &c.name,
                    format!("{}-{}.crate", &c.name, &c.vers).as_str(),
                )
                .unwrap();
            }
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
            error!("{:?}", err);
        }
    }
}

/// download file from remote and calculate it's hash
/// return true if download and success, return flase if file already exists
pub fn download_file(
    url: &str,
    path: &Path,
    check_sum: Option<&str>,
    is_override: bool,
) -> Result<bool, FreighterError> {
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
                return download_with_folder(url, path, "!!![REMOVED DOWNLOAD] \t\t ");
            }
        } else if !is_override {
            info!(
                "file exist but not pass check_sum, skiping download {}",
                path.display()
            );
            return Ok(false);
        }
    }
    download_with_folder(url, path, "&&&[NEW] \t\t ")
}

/// return false if connect success but download failed
pub fn download_with_folder(url: &str, path: &Path, msg: &str) -> Result<bool, FreighterError> {
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

/// upload file to s3
pub fn upload_file(file: &str, folder: &str, filename: &str) -> FreightResult {
    // cargo download url is https://crates.rust-lang.pub/crates/{name}/{version}/download
    //

    // Upload to the Digital Ocean Spaces with s3cmd
    // URL: s3://rust-lang/crates/{}/{}
    // cmd: s3cmd put {file} s3://rust-lang/crates/{folder}/{file-name} --acl-public
    // cmd: s3cmd put {file} s3://rust-lang/crates/{folder}/{file-name} --acl-public --no-mime-magic
    // cmd: s3cmd put {file} s3://rust-lang/crates/{folder}/{file-name} --acl-public --no-mime-magic --guess-mime-type
    // cmd: s3cmd put {file} s3://rust-lang/crates/{folder}/{file-name} --acl-public --no-mime-magic --guess-mime-type --add-header="Content-Type: application/octet-stream"
    let status = std::process::Command::new("s3cmd")
        .arg("put")
        .arg(file)
        .arg(format!("s3://rust-lang/crates/{}/{}", folder, filename))
        .arg("--acl-public")
        .status()
        .expect("failed to execute process");
    if !status.success() {
        return Err(FreighterError::code(status.code().unwrap()));
    }
    Ok(())
}
