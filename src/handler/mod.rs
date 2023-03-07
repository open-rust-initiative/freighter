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