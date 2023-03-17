//! provide common functionality for cloud operation
//!
//!

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use threadpool::ThreadPool;
use walkdir::WalkDir;

use crate::{errors::FreightResult, handler::crates_file::is_not_hidden};

use self::s3::S3cmd;

pub mod s3;

/// provide a common file upload interface
pub trait CloudStorage {
    /// upload a single file to target storage
    fn upload_file(&self, file_path: &Path, s3_path: &str, bucket: &str) -> FreightResult;

    /// this operation will upload all files in folder
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult;
}

// this method is used to handle 'uplaod' subcommand for uplaod all files to obs server
pub fn upload_with_pool(
    download_threads: usize,
    path: PathBuf,
    bucket_name: String,
    cloud_storage: S3cmd,
) -> FreightResult {
    let pool = ThreadPool::new(download_threads);
    let cloud = Arc::new(cloud_storage);
    let bucket_name = format!(
        "{}/{}",
        bucket_name,
        path.file_name().unwrap().to_str().unwrap()
    );
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_entry(is_not_hidden)
        .filter_map(|v| v.ok())
        .for_each(|x| {
            let bucket_name = bucket_name.clone();
            let cloud_in = cloud.clone();
            pool.execute(move || {
                let path = x.path();
                cloud_in
                    .upload_folder(path.to_str().unwrap(), &bucket_name)
                    .unwrap();
            });
        });
    pool.join();
    tracing::info!("sync ends with {} task failed", pool.panic_count());
    Ok(())
}

pub fn upload_single_dir<T: CloudStorage>(
    path: PathBuf,
    crates_name: String,
    bucket_name: String,
    cloud_storage: T,
) {
    let bucket_name = format!(
        "{}/{}",
        bucket_name,
        path.file_name().unwrap().to_str().unwrap()
    );
    tracing::info!("bucket_path: {}", bucket_name);
    cloud_storage
        .upload_folder(path.join(crates_name).to_str().unwrap(), &bucket_name)
        .unwrap();
}
