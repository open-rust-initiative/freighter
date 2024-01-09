//!
//!
//!
//!
//!

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// parse config from file
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default = "default_value_for_path")]
    pub index_path: PathBuf,
    #[serde(default = "default_value_for_path")]
    pub crates_path: PathBuf,
    #[serde(default = "default_value_for_path")]
    pub log_path: PathBuf,
    #[serde(default = "default_value_for_path")]
    pub rustup_path: PathBuf,
    #[serde(default = "default_value_for_path")]
    pub dist_path: PathBuf,
    
    pub crates: CratesConfig,
    pub rustup: RustUpConfig,
    pub log: LogConfig,
    pub proxy: ProxyConfig,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LogConfig {
    #[serde(deserialize_with = "path_option_from_str")]
    pub log_path: Option<PathBuf>,
    pub encoder: String,
    pub level: String,
    pub limit: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CratesConfig {
    #[serde(deserialize_with = "path_option_from_str")]
    pub index_path: Option<PathBuf>,
    #[serde(deserialize_with = "path_option_from_str")]
    pub crates_path: Option<PathBuf>,
    pub index_domain: String,
    pub domain: String,
    pub download_threads: usize,
    pub serve_domains: Option<Vec<String>>,
    pub serve_index: Option<String>,
}

/// config for rustup mirror sync
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RustUpConfig {
    #[serde(deserialize_with = "path_option_from_str")]
    pub rustup_path: Option<PathBuf>,
    #[serde(deserialize_with = "path_option_from_str")]
    pub dist_path: Option<PathBuf>,
    pub domain: String,
    pub download_threads: usize,
    pub sync_stable_versions: Vec<String>,
    pub sync_nightly_days: i64,
    pub sync_beta_days: i64,
    pub serve_domains: Option<Vec<String>>,
    pub history_version_start_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProxyConfig {
    pub enable: bool,
    pub git_index_proxy: String,
    pub download_proxy: String,
}

// deserialize a string from a TOML file into an Option<PathBuf>
fn path_option_from_str<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(if s.is_empty() {
        None
    } else {
        Some(Path::new(&s).to_path_buf())
    })
}

fn default_value_for_path() -> PathBuf {
    PathBuf::new()
}

///
impl Config {
    pub fn new() -> Config {
        Config {
            index_path: PathBuf::new(),
            crates_path: PathBuf::new(),
            log_path: PathBuf::new(),
            rustup_path: PathBuf::new(),
            dist_path: PathBuf::new(),
            rustup: RustUpConfig::default(),
            crates: CratesConfig::default(),
            log: LogConfig::default(),
            proxy: ProxyConfig::default(),
        }
    }

    pub fn load(&self, config_parent: Option<PathBuf>) -> Config {
        let config_path = format_path(&config_parent, "config.toml");
        let mut config = Self::read_config_or_init(&config_path);

        config.index_path = format_path(&config.crates.index_path, "crates.io-index");
        config.crates_path = format_path(&config.crates.crates_path, "crates");
        config.log_path = format_path(&config.log.log_path, "log");
        config.rustup_path = format_path(&config.rustup.rustup_path, "rustup");
        config.dist_path = format_path(&config.rustup.dist_path, "dist");
        config
    }

    // read from config file, default config file will be created if not exist
    pub fn read_config_or_init(config_path: &Path) -> Config {
        let content = match fs::read_to_string(config_path) {
            Ok(content) => content,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    if let Some(parent) = config_path.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent).unwrap();
                        }
                    }
                    //rewrite config.default.toml under config_path
                    fs::write(config_path, include_str!("config.default.toml")).unwrap();
                    fs::read_to_string(config_path).unwrap()
                }
                other_error => panic!("Can't read config file: {}", other_error),
            },
        };
        match toml::from_str(&content) {
            Ok(config) => config,
            Err(err) => panic!("Config file doesn't match, maybe it's outdated or you have provided a invalid value, 
            you can manually delete it and try again.
            Caused by {}", err),
        }
    }
}

pub fn format_path(config_path: &Option<PathBuf>, name: &str) -> PathBuf {
    let default_dir = dirs::home_dir().unwrap().join("freighter");
    let path = match config_path {
        Some(path) => path,
        None => &default_dir,
    };
    let path_str = path.to_str().unwrap();
    if !path_str.contains("freighter") {
        return path.join("freighter").join(name);
    } else if !path_str.contains(name) {
        return path.join(name);
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::format_path;

    #[test]
    fn test_format_path() {
        // test config path
        let default = dirs::home_dir().unwrap();
        assert_eq!(format_path(&None, "config.toml"), default.join("freighter/config.toml"));
        assert_eq!(format_path(&Some("/tmp".into()), "config.toml"), PathBuf::from("/tmp/freighter/config.toml"));

        // test index path
        assert_eq!(format_path(&None, "index"), default.join("freighter/index"));
        assert_eq!(format_path(&Some("/tmp".into()), "index"), PathBuf::from("/tmp/freighter/index"));
        assert_eq!(format_path(&Some("/tmp/freighter".into()), "index"), PathBuf::from("/tmp/freighter/index"));
        assert_eq!(format_path(&Some("/tmp/freighter/index".into()), "index"), PathBuf::from("/tmp/freighter/index"));
    }
}
