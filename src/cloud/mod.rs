//! provide common functionality for cloud operation
//!
//!

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use threadpool::ThreadPool;
use walkdir::WalkDir;

use crate::{crates::crates_file::is_not_hidden, errors::FreightResult};

use self::s3::S3cmd;

pub mod s3;

/// provide a common file upload interface
pub trait CloudStorage {
    /// upload a single file to target storage
    fn upload_file(&self, file_path: &Path, s3_path: &str, bucket: &str) -> FreightResult;

    /// this operation will upload all files in folder
    fn upload_folder(&self, folder: &str, bucket: &str) -> FreightResult;
}

pub fn upload_with_pool(
    download_threads: usize,
    path: PathBuf,
    bucket_name: String,
    cloud_storage: S3cmd,
) -> FreightResult {
    let pool = ThreadPool::new(download_threads);
    // let cloud_storage = S3cmd::default();
    let cloud = Arc::new(Mutex::new(cloud_storage));
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_entry(is_not_hidden)
        .filter_map(|v| v.ok())
        .for_each(|x| {
            let bucket_name = bucket_name.clone();
            let cloud = Arc::clone(&cloud);
            pool.execute(move || {
                let storage = cloud.lock().unwrap();
                let path = x.path();
                storage
                    .upload_folder(
                        path.to_str().unwrap(),
                        &format!("{}/{}", bucket_name, "crates"),
                    )
                    .unwrap();
            });
        });
    pool.join();
    tracing::info!("sync ends with {} task failed", pool.panic_count());
    Ok(())
}
