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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub sync_stable_versions: Vec<String>,
    pub sync_nightly_days: i64,
    pub sync_beta_days: i64,
    pub crates_source: String,
}

///
impl Config {
    pub fn new() -> Config {
        Config {
            sync_stable_versions: [].to_vec(),
            sync_nightly_days: 30,
            sync_beta_days: 30,
            crates_source: String::from("https://static.crates.io/crates"),
        }
    }

    pub fn config_path(home_path: &Path) -> PathBuf {
        home_path.join("freighter/config.toml")
    }

    pub fn load(&self, home_path: &Path) -> Config {
        let config_path = Self::config_path(home_path);
        Self::get_config(&config_path)
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
            Err(_) => panic!("Config file doesn't match, maybe it's outdated or you have provided a invalid value, you can manaully delete it and try again"),
        }
    }
}
