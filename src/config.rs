//!
//!
//!
//!
//!

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use serde::{Deserialize, Serialize};

/// 
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub work_dir: Option<PathBuf>,
    pub crates: CratesConfig,
    pub rustup: RustUpConfig,
    pub log: LogConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LogConfig {
    pub encoder: String,
    pub level: String,
    pub size: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CratesConfig {
    pub index_domain: String,
    pub domain: String,
    pub download_threads: usize,
}

/// config for rustup mirror sync 
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RustUpConfig {
    pub domain: String,
    pub download_threads: usize,
    pub sync_stable_versions: Vec<String>,
    pub sync_nightly_days: i64,
    pub sync_beta_days: i64,
}

///
impl Config {
    pub fn new() -> Config {
        Config {
            work_dir: None,
            rustup: RustUpConfig::default(),
            crates: CratesConfig::default(),
            log: LogConfig::default(),
        }
    }

    pub fn format_path(home_path: &Path) -> PathBuf {
        home_path.join("freighter/config.toml")
    }

    pub fn load(&self, path: &Path) -> Config {
        let path = Self::format_path(path);
        Self::get_config(&path)
    }

    // read channel list from config file, if config file don't exist then it will be created from default file
    pub fn get_config(config_path: &Path) -> Config {
        let content = match fs::read_to_string(config_path) {
            Ok(content) => content,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    if let Some(parent) = config_path.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent).unwrap();
                        }
                    }
                    fs::write(config_path, include_str!("config.default.toml")).unwrap();
                    fs::read_to_string(config_path).unwrap()
                }
                other_error => panic!("Can't read config file: {}", other_error),
            },
        };
        match toml::from_str(&content) {
            Ok(config) => config,
            Err(err) => panic!("Config file doesn't match, maybe it's outdated or you have provided a invalid value, 
            you can manaully delete it and try again. Caused by {}", err),
        }
    }
}
