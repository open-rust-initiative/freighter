use std::{io, path::Path, fs::{File, self}};
use sha2::{Sha256, Digest};
use crate::errors::{FreighterError, FreightResult};


// download remote sha file and then download file for hash check
pub fn download_file_with_sha(url: &str, file_folder: &Path, file_name: &str) -> Result<bool, FreighterError> {
    let sha_url = format!("{}{}", url, ".sha256");
    let sha_name = format!("{}{}", file_name, ".sha256");
    let sha_path = file_folder.join(&sha_name);
    //always update sha256 file
    download_file(&sha_url, &sha_path, None, true).unwrap();
    match fs::read_to_string(&sha_path) {
        Ok(content) => {
            let sha256 = &content[..64];
            download_file(&url, &file_folder.join(file_name), Some(sha256), false).unwrap();
        },
        Err(_) => return Ok(false),
    };
    Ok(true)
}

/// download file from remote and calculate it's hash, return true if download and success
pub fn download_file(url: &str, path: &Path, check_sum: Option<&str>, is_override: bool) -> Result<bool, FreighterError> {
    if path.is_file() && path.exists() {
        let mut hasher = Sha256::new();
        let mut f = File::open(path)?;
        io::copy(&mut f, &mut hasher)?;
        let result = hasher.finalize();
        let hex = format!("{:x}", result);

        //if need to calculate hash
        if check_sum.is_some() {
            if hex == check_sum.unwrap() {
                println!("###[ALREADY] \t{:?}", f);
                return Ok(false);
            } else {
                println!("!!![REMOVE] \t\t {:?} !", f);
                fs::remove_file(path)?;
                generate_folder_and_file(url, path, "!!![REMOVED DOWNLOAD] \t\t ").unwrap();
            }
        } else if !is_override {
            println!("file exist but not pass check_sum, skiping download {}", path.display());
            return Ok(false);
        }   
    }
    generate_folder_and_file(url, path, "&&&[NEW] \t\t ").unwrap();
    Ok(true)
}

pub fn generate_folder_and_file(url: &str, path: &Path, msg: &str) -> FreightResult {
    // generate parent folder if unexist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).unwrap();
        }
    }
    let mut resp = reqwest::blocking::get(url).unwrap();
    if resp.status() == 200 {
        let mut out = File::create(path).unwrap();
        io::copy(&mut resp, &mut out).unwrap();
        println!("{} {:?}", msg, out);
    }
    Ok(())
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