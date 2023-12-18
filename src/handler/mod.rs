//! crates mod contains index, crates and rustup
//!
//!
//!
//!
//!

pub mod channel;
pub mod crates_file;
pub mod index;
pub mod rustup;

#[derive(Clone, Default, Debug)]
pub enum DownloadMode {
    Init,
    // indicates this operation is fix error downloads
    Fix,
    #[default]
    Increment,
}
impl DownloadMode {
    pub fn new(init: bool, fix: bool) -> Self {
        if init {
            DownloadMode::Init
        } else if fix {
            DownloadMode::Fix
        } else {
            DownloadMode::Increment
        }
    }
}

pub mod utils {

    // the path rules of crates index file
    pub fn index_suffix(name: &str) -> String {
        match name.len() {
            1..=2 => format!("{}/{}", name.len(), name),
            3 => format!("{}/{}/{}", name.len(), &name[0..1], name),
            _ => format!("{}/{}/{}", &name[0..2], &name[2..4], name),
        }
    }
}
