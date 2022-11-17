use log::info;

use crate::errors::{FreightResult, FreighterError};

/// provide a common file upload interface
pub trait CloudStorage {
    /// upload a single file to target storage
    // fn upload(&self, bucket: &str) -> FreightResult;

    /// this operation will upload all files in folder
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult;
}

#[derive(Default)]
pub struct S3cmd {}

impl CloudStorage for S3cmd {
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult {
        info!("trying to upload folder {} to s3", folder);
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
}
