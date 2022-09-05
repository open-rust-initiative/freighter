///
///
///
/// Some codes are from [git2-rs](https://github.com/rust-lang/git2-rs)'s clone (example)[https://github.com/rust-lang/git2-rs/blob/master/examples/clone.rs].
///
/// TODO
/// - [ ] 1. Link the [CrateIndex] with [sync] subcommand
/// - [ ] 2. Add https://github.com/rust-lang/crates.io-index.git as default url value
/// - [ ] 3. Add check the destination path is empty
/// - [ ] 4. Add check the destination path is a git repository
/// - [ ] 5. Add check the destination path is a crates-io index
/// - [ ] 6. If the destination path is a git repository and is a crate-io index, run pull instead of clone
/// - [ ] 7. Add a flag for `enable` or `disable` the progress bar
/// - [ ] 8. Change the test index git repo with local git repository for test performance

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{FetchOptions, Progress, RemoteCallbacks};
use std::cell::RefCell;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use url::Url;

use crate::errors::FreightResult;

/// `CrateIndex` is a wrapper `Git Repository` that crates-io index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateIndex {
    pub url: Url,
    pub path: PathBuf,
}

///
pub struct State {
    pub progress: Option<Progress<'static>>,
    pub total: usize,
    pub current: usize,
    pub path: Option<PathBuf>,
    pub newline: bool,
}

///
///
impl CrateIndex {
    /// Create a new `CrateIndex` from a `Url`.
    pub fn new(url: Url, path: PathBuf) -> Self {
        Self { url, path}
    }

    /// Get the `url` of this `CrateIndex`.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get the `path` of this `CrateIndex`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Clone the `CrateIndex` to a local directory.
    pub fn clone(&self) -> FreightResult {
        let state = RefCell::new(State {
            progress: None,
            total: 0,
            current: 0,
            path: None,
            newline: false,
        });

        let mut cb = RemoteCallbacks::new();
        cb.transfer_progress(|stats| {
            let mut state = state.borrow_mut();
            state.progress = Some(stats.to_owned());
            print(&mut *state);
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            print(&mut *state);
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        RepoBuilder::new()
            .fetch_options(fo)
            .with_checkout(co)
            .clone(self.url.as_ref(), self.path.as_path())?;
        println!();

        Ok(())
    }
}

///
///
///
fn print(state: &mut State) {
    let stats = state.progress.as_ref().unwrap();
    let network_pct = (100 * stats.received_objects()) / stats.total_objects();
    let index_pct = (100 * stats.indexed_objects()) / stats.total_objects();
    let co_pct = if state.total > 0 {
        (100 * state.current) / state.total
    } else {
        0
    };

    let kb = stats.received_bytes() / 1024;

    if stats.received_objects() == stats.total_objects() {
        if !state.newline {
            println!();
            state.newline = true;
        }
        print!(
            "Resolving deltas {}/{}\r",
            stats.indexed_deltas(),
            stats.total_deltas()
        );
    } else {
        print!(
            "net {:3}% ({:4} kb, {:5}/{:5})  /  idx {:3}% ({:5}/{:5})  \
             /  chk {:3}% ({:4}/{:4}) {}\r",
            network_pct,
            kb,
            stats.received_objects(),
            stats.total_objects(),
            index_pct,
            stats.indexed_objects(),
            stats.total_objects(),
            co_pct,
            state.current,
            state.total,
            state
                .path
                .as_ref()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        )
    }

    io::stdout().flush().unwrap();
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn test_clone() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/tests/fixtures/crates-io-index");

        let mut index = super::CrateIndex::new(
            url::Url::parse("https://github.com/rust-lang/crates.io-index.git").unwrap(),
            path
        );

        index.clone().unwrap();

    }
}