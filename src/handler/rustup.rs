//!
//!
//!
//!
//!
//!

use std::path::PathBuf;

use threadpool::ThreadPool;

use crate::{
    config::ProxyConfig,
    config::RustUpConfig,
    download::{download_and_check_hash, download_file_with_sha, DownloadOptions},
    errors::FreightResult,
};

//rustup support platforms, see https://doc.rust-lang.org/beta/rustc/platform-support.html
const PLATFORMS: &[&str] = &[
    "aarch64-linux-android",
    "aarch64-unknown-linux-gnu",
    "arm-linux-androideabi",
    "arm-unknown-linux-gnueabi",
    "arm-unknown-linux-gnueabihf",
    "armv7-linux-androideabi",
    "armv7-unknown-linux-gnueabihf",
    "i686-apple-darwin",
    "i686-linux-android",
    "i686-unknown-linux-gnu",
    "mips-unknown-linux-gnu",
    "mips64-unknown-linux-gnuabi64",
    "mips64el-unknown-linux-gnuabi64",
    "mipsel-unknown-linux-gnu",
    "powerpc-unknown-linux-gnu",
    "powerpc64-unknown-linux-gnu",
    "powerpc64le-unknown-linux-gnu",
    "s390x-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-linux-android",
    "x86_64-unknown-freebsd",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-netbsd",
    "i686-pc-windows-gnu",
    "i686-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
];

#[derive(Default, Debug, Clone)]
pub struct RustUpOptions {
    pub config: RustUpConfig,

    pub proxy: ProxyConfig,

    pub rustup_path: PathBuf,
}

/// entrance function
pub fn sync_rustup_init(opts: &RustUpOptions) -> FreightResult {
    let download_url = format!("{}/rustup/release-stable.toml", opts.config.domain);
    let file = opts.rustup_path.join("release-stable.toml");
    let down_opts = &DownloadOptions {
        proxy: opts.proxy.clone(),
        url: download_url,
        path: file,
    };

    download_and_check_hash(down_opts, None, true).unwrap();
    let pool = ThreadPool::new(opts.config.download_threads);
    PLATFORMS.iter().for_each(|platform| {
        let rustup_path = opts.rustup_path.clone();
        let file_name = if platform.contains("windows") {
            "rustup-init.exe".to_owned()
        } else {
            "rustup-init".to_owned()
        };
        let domain = opts.config.domain.clone();
        let proxy = opts.proxy.clone();
        pool.execute(move || {
            let download_url = format!("{}/rustup/dist/{}/{}", domain, platform, file_name);
            let folder = rustup_path.join("dist").join(platform);
            download_file_with_sha(&download_url, &folder, &file_name, &proxy).unwrap();
        });
    });
    pool.join();

    Ok(())
}
