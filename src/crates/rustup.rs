use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use threadpool::ThreadPool;

use crate::{
    download::{download_file, download_file_with_sha},
    errors::{FreightResult, FreighterError},
};

use super::index::CrateIndex;

const RUSTUP_MIRROR: &str = "https://static.rust-lang.org";

//rustup platforms arr
const PLATFORMS: [&str; 86] = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-fuchsia",
    "aarch64-linux-android",
    "aarch64-pc-windows-msvc",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "aarch64-unknown-none",
    "aarch64-unknown-none-softfloat",
    "armebv7r-none-eabi",
    "armebv7r-none-eabihf",
    "arm-linux-androideabi",
    "arm-unknown-linux-gnueabi",
    "arm-unknown-linux-gnueabihf",
    "arm-unknown-linux-musleabi",
    "arm-unknown-linux-musleabihf",
    "armv5te-unknown-linux-gnueabi",
    "armv5te-unknown-linux-musleabi",
    "armv7a-none-eabi",
    "armv7-linux-androideabi",
    "armv7r-none-eabi",
    "armv7r-none-eabihf",
    "armv7-unknown-linux-gnueabi",
    "armv7-unknown-linux-gnueabihf",
    "armv7-unknown-linux-musleabi",
    "armv7-unknown-linux-musleabihf",
    "asmjs-unknown-emscripten",
    "i586-pc-windows-msvc",
    "i586-unknown-linux-gnu",
    "i586-unknown-linux-musl",
    "i686-linux-android",
    "i686-pc-windows-gnu",
    "i686-pc-windows-msvc",
    "i686-unknown-freebsd",
    "i686-unknown-linux-gnu",
    "i686-unknown-linux-musl",
    "mips64el-unknown-linux-gnuabi64",
    "mips64el-unknown-linux-muslabi64",
    "mips64-unknown-linux-gnuabi64",
    "mips64-unknown-linux-muslabi64",
    "mipsel-unknown-linux-gnu",
    "mipsel-unknown-linux-musl",
    "mips-unknown-linux-gnu",
    "mips-unknown-linux-musl",
    "nvptx64-nvidia-cuda",
    "powerpc64le-unknown-linux-gnu",
    "powerpc64-unknown-linux-gnu",
    "powerpc-unknown-linux-gnu",
    "riscv32imac-unknown-none-elf",
    "riscv32imc-unknown-none-elf",
    "riscv32i-unknown-none-elf",
    "riscv64gc-unknown-linux-gnu",
    "riscv64gc-unknown-none-elf",
    "riscv64imac-unknown-none-elf",
    "s390x-unknown-linux-gnu",
    "sparc64-unknown-linux-gnu",
    "sparcv9-sun-solaris",
    "thumbv6m-none-eabi",
    "thumbv7em-none-eabi",
    "thumbv7em-none-eabihf",
    "thumbv7m-none-eabi",
    "thumbv7neon-linux-androideabi",
    "thumbv7neon-unknown-linux-gnueabihf",
    "thumbv8m.base-none-eabi",
    "thumbv8m.main-none-eabi",
    "thumbv8m.main-none-eabihf",
    "wasm32-unknown-emscripten",
    "wasm32-unknown-unknown",
    "wasm32-wasi",
    "x86_64-apple-darwin",
    "x86_64-apple-ios",
    "x86_64-fortanix-unknown-sgx",
    "x86_64-fuchsia",
    "x86_64-linux-android",
    "x86_64-pc-solaris",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-sun-solaris",
    "x86_64-unknown-freebsd",
    "x86_64-unknown-illumos",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnux32",
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-netbsd",
    "x86_64-unknown-redox",
];

#[derive(Debug, Deserialize)]
pub struct Channel {
    #[serde(alias = "manifest-version")]
    pub manifest_version: String,
    pub date: String,
    pub pkg: HashMap<String, Pkg>,
}

#[derive(Debug, Deserialize)]
pub struct Pkg {
    pub version: String,
    pub target: HashMap<String, Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xz_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xz_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub sync_stable_versions: Vec<String>,
    pub sync_nightly_days: usize,
}

/// entrance function
pub fn sync_rustup(index: CrateIndex) -> FreightResult {
    sync_rustup_init(&index)?;
    // sync latest stable,beta and nightly
    sync_channel(&index, "stable")?;
    sync_channel(&index, "beta")?;
    sync_channel(&index, "nightly")?;
    // sync specified version by config file
    let config = get_config(&index).unwrap();
    config.sync_stable_versions.iter().for_each(|channel| {
        sync_channel(&index, channel).unwrap();
    });
    Ok(())
}

/// sync rustup init file from static.rust-lang
pub fn sync_rustup_init(index: &CrateIndex) -> FreightResult {
    let download_url = format!("{}/rustup/release-stable.toml", RUSTUP_MIRROR);
    let file = index.rustup_path.join(format!("release-stable.toml"));
    download_file(&download_url, &file, None, true).unwrap();
    let pool = ThreadPool::new(index.thread_count);
    PLATFORMS.iter().for_each(|platform| {
        let rustup_path = index.rustup_path.clone();
        let file_name = if platform.contains("windows") {
            "rustup-init.exe".to_owned()
        } else {
            "rustup-init".to_owned()
        };
        pool.execute(move || {
            let download_url = format!("{}/rustup/dist/{}/{}", RUSTUP_MIRROR, platform, file_name);
            let folder = rustup_path.join("dist").join(platform);
            download_file_with_sha(&download_url, &folder, &file_name).unwrap();
        });
    });
    pool.join();
    Ok(())
}

// sync rust toolchain by channel
pub fn sync_channel(index: &CrateIndex, channel: &str) -> FreightResult {
    let channel_name = format!("channel-rust-{}.toml", channel);
    let channel_url = format!("{}/dist/{}", RUSTUP_MIRROR, channel_name);
    download_file_with_sha(&channel_url, &index.dist_path, &channel_name).unwrap();
    let pool = ThreadPool::new(index.thread_count);
    // parse_channel_file and download;
    let file_list = parse_channel_file(&index.dist_path.join(channel_name)).unwrap();
    file_list.into_iter().for_each(|(url, hash)| {
        // example: https://static.rust-lang.org/dist/2022-11-03/rust-1.65.0-i686-pc-windows-gnu.msi
        // remove url prefix "https://static.rust-lang.org/dist"
        let path: PathBuf = std::iter::once(index.dist_path.to_owned())
            .chain(url.split("/").map(PathBuf::from).collect::<Vec<PathBuf>>()[4..].to_owned())
            .collect();
        pool.execute(move || {
            download_file(&url, &path, Some(&hash), false).unwrap();
        });
    });
    pool.join();
    Ok(())
}

pub fn parse_channel_file(path: &Path) -> Result<Vec<(String, String)>, FreighterError> {
    let content = fs::read_to_string(path).unwrap();
    // println!("{}", &content[..64]);
    let channel: Channel = toml::from_str(&content).unwrap();
    let res: Vec<(String, String)> = channel
        .pkg
        .into_iter()
        .flat_map(|(_, pkg)| {
            pkg.target
                .into_iter()
                .flat_map(|(_, target)| -> Vec<(String, String)> {
                    let mut result: Vec<(String, String)> = Vec::new();
                    if target.xz_url.is_some() && target.xz_hash.is_some() {
                        result.push((target.xz_url.unwrap(), target.xz_hash.unwrap()));
                    }
                    result
                })
        })
        .collect();
    Ok(res)
}

pub fn get_config(index: &CrateIndex) -> Result<Config, FreighterError> {
    let content = match fs::read_to_string(&index.config_path) {
        Ok(content) => content,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                if let Some(parent) = index.config_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).unwrap();
                    }
                }
                fs::write(&index.config_path, include_str!("config.default.toml"))?;
                fs::read_to_string(&index.config_path).unwrap()
            }
            other_error => panic!("Can't read config file: {}", other_error),
        },
    };
    let config: Config = toml::from_str(&content).unwrap();
    Ok(config)
}
