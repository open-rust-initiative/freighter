//!
//!
//!
//!
//!
//!

use log::info;
use std::path::Path;

use log::debug;

use crate::errors::{FreightResult, FreighterError};

/// provide a common file upload interface
pub trait CloudStorage {
    /// upload a single file to target storage
    fn upload_file(&self, file_path: &Path, s3_path: &str, bucket: &str) -> FreightResult;

    /// this operation will upload all files in folder
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult;
}

#[derive(Default)]
pub struct S3cmd {}

impl CloudStorage for S3cmd {
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult {
        debug!("trying to upload folder {} to s3", folder);
        let status = std::process::Command::new("s3cmd")
            .arg("sync")
            .arg(folder)
            .arg(format!("s3://{}/", bucket))
            .arg("--acl-public")
            .status()
            .expect("failed to execute s3cmd sync");
        if !status.success() {
            return Err(FreighterError::code(status.code().unwrap()));
        }
        Ok(())
    }

    fn upload_file(&self, file_path: &Path, s3_path: &str, bucket: &str) -> FreightResult {
        // cargo download url is https://crates.rust-lang.pub/crates/{name}/{version}/download
        //

        // Upload to the Digital Ocean Spaces with s3cmd
        // URL: s3://rust-lang/crates/{}/{}
        // cmd: s3cmd put {file_path} s3://rust-lang/crates/{s3_path} --acl-public
        // cmd: s3cmd put {file_path} s3://rust-lang/crates/{s3_path} --acl-public --no-mime-magic
        // cmd: s3cmd put {file_path} s3://rust-lang/crates/{s3_path} --acl-public --no-mime-magic --guess-mime-type
        // cmd: s3cmd put {file_path} s3://rust-lang/crates/{s3_path} --acl-public --no-mime-magic --guess-mime-type --add-header="Content-Type: application/octet-stream"
        let s3_full_path = format!("s3://{}/{}", bucket, s3_path);
        debug!("s3_full_path: {}", s3_full_path);
        let status = std::process::Command::new("s3cmd")
            .arg("put")
            .arg(file_path)
            .arg(s3_full_path)
            .arg("--acl-public")
            .status()
            .expect("failed to execute process");
        if !status.success() {
            return Err(FreighterError::code(status.code().unwrap()));
        }
        Ok(())
    }
}
