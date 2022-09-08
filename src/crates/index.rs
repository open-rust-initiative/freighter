///
///
/// ### References Codes
///
/// - [git2-rs](https://github.com/rust-lang/git2-rs)'s clone (example)[https://github.com/rust-lang/git2-rs/blob/master/examples/clone.rs].
/// - [crates.io](https://github.com/rust-lang/crates.io)'s [structs](https://github.com/rust-lang/crates.io/blob/master/cargo-registry-index/lib.rs)
///
/// TODO
/// - [ ] 1. Link the `CrateIndex` with `sync` subcommand
/// - [ ] 2. Add https://github.com/rust-lang/crates.io-index.git as default url value
/// - [ ] 3. Add check the destination path is empty
/// - [ ] 4. Add check the destination path is a git repository
/// - [ ] 5. Add check the destination path is a crates-io index
/// - [ ] 6. If the destination path is a git repository and is a crate-io index, run pull instead of clone
/// - [ ] 7. Add a flag for `enable` or `disable` the progress bar
/// - [ ] 8. Change the test index git repo with local git repository for test performance

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{FetchOptions, Progress, RemoteCallbacks, Repository};

use url::Url;
use walkdir::{DirEntry, WalkDir};
use serde::{Deserialize, Serialize};
use rand::Rng;
use sha2::{Digest, Sha256};

use std::collections::BTreeMap;
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Write};
use std::path::{Path, PathBuf};
use std::{fs, thread};
use std::time::Duration;
use std::{env, process};

use crate::errors::{FreighterError, FreightResult};
use crate::crates::{pull};

/// `CrateIndex` is a wrapper `Git Repository` that crates-io index.
///
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateIndex {
    pub url: Url,
    pub path: PathBuf,
}

///
///
///
pub struct State {
    pub progress: Option<Progress<'static>>,
    pub total: usize,
    pub current: usize,
    pub path: Option<PathBuf>,
    pub newline: bool,
}

impl CrateIndex {
    pub const CRATE_REGISTRY: [&'static str; 3] = ["https://github.com/rust-lang/crates.io-index.git","",""];
}

impl Default for CrateIndex {
    fn default() -> CrateIndex {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/tests/fixtures/crates-io-index");
        CrateIndex{
            url: Url::parse(CrateIndex::CRATE_REGISTRY[0]).unwrap(),
            path: path,
        }
    }
}

///
///
///
#[derive(Serialize, Deserialize, Debug)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<BTreeMap<String, Vec<String>>>,
    pub yanked: Option<bool>,
    #[serde(default)]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u32>,
}

///
///
///
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

///
///
///
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

///
///
///
impl CrateIndex {
    /// Create a new `CrateIndex` from a `Url`.
    pub fn new(url: Url, path: PathBuf, buf: PathBuf) -> Self {
        Self { url, path}
    }

    /// Get the `path` of this `CrateIndex`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Clone the `CrateIndex` to a local directory.
    ///
    ///
    pub fn clone(&self) -> FreightResult {
        println!("Starting git clone...");
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

    /// https://github.com/rust-lang/crates.io-index/blob/master/.github/workflows/update-dl-url.yml
    ///
    /// ```YAML
    ///env:
    ///   URL_api: "https://crates.io/api/v1/crates"
    ///   URL_cdn: "https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
    ///   URL_s3_primary: "https://crates-io.s3-us-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
    ///   URL_s3_fallback: "https://crates-io-fallback.s3-eu-west-1.amazonaws.com/crates/{crate}/{crate}-{version}.crate"
    /// ```
    pub fn downloads(&self, path: PathBuf) -> FreightResult {
        let mut urls = Vec::new();

        WalkDir::new(self.path()).into_iter()
            .filter_entry(|e| is_not_hidden(e))
            .filter_map(|v| v.ok())
            .for_each(|x| {
                if x.file_type().is_file() && x.path().extension().unwrap_or_default() != "json" {
                    let input = File::open(x.path()).unwrap();
                    let buffered = BufReader::new(input);

                    for line in buffered.lines() {
                        let line = line.unwrap();
                        let c: Crate = serde_json::from_str(&line).unwrap();

                        let url = format!("https://static.crates.io/crates/{}/{}-{}.crate", c.name,c.name, c.vers);
                        let file = path.join(format!("{}-{}.crate", c.name, c.vers));

                        urls.push((url, file.to_str().unwrap().to_string(), c.cksum));
                    }
                }
            });

        let mut i = 0;
        for c in urls {
            let (url, file, cksum) = c;

            // https://github.com/RustScan/RustScan/wiki/Thread-main-paniced-at-too-many-open-files
            if i % 60 == 0 {
                let mut rng = rand::thread_rng();
                thread::sleep(Duration::from_secs(rng.gen_range(1..5)));
            }

            thread::spawn(move || {
                let p = Path::new(&file);

                if p.is_file() == true && p.exists() == true {
                    let mut hasher = Sha256::new();
                    let mut f = File::open(p).unwrap();
                    io::copy(&mut f, &mut hasher).unwrap();
                    let result = hasher.finalize();
                    let hex = format!("{:x}", result);

                    if hex == cksum {
                        println!("Already downloaded: {:?}", p);
                    } else {
                        println!("The file {} SHA256 value \"{}\" is wrong, will remove and download again!", file, hex);
                        fs::remove_file(p).unwrap();

                        let mut resp = reqwest::blocking::get(url).unwrap();
                        let mut out = File::create(file).unwrap();
                        io::copy(&mut resp, &mut out).unwrap();

                        println!("Remove and new downloaded: {:?}", out);
                    }
                } else {
                    let mut resp = reqwest::blocking::get(url).unwrap();
                    let mut out = File::create(file).unwrap();
                    io::copy(&mut resp, &mut out).unwrap();

                    println!("New downloaded: {:?}", out);
                }

            });

            i += 1;
        }

        Ok(())
    }
}

///
fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with("."))
        .unwrap_or(false)
}

///
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


pub fn run(index: CrateIndex) -> FreightResult {
    if exist_file(&index) {
        if let Err(e) = git_dir_check(&index) {
            e.print();
        }
    } else {
        index.clone()?;
    }
    Ok(())
}

pub fn exist_file(index: &CrateIndex) -> bool {
    Path::new(index.path.as_path()).exists()
}

pub fn git_dir_check(index: &CrateIndex) -> FreightResult {
    let path = index.path.to_str().map(|s| &s[..]).unwrap_or(".");
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => panic!("Traget path is not a git repository: {}", e),
    };
    let remote_name = &String::from("origin");
    let mut remote = repo.find_remote(remote_name).unwrap();
    let url = remote.url().unwrap(); 
    println!("current remote registry is: {}", url);
    if CrateIndex::CRATE_REGISTRY.contains(&url) {

        pull::run(&repo, &mut remote)
    } else {
        panic!("Traget url is not a crates index: {}", url)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn test_clone() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/tests/fixtures/crates.io-index");

        let index = super::CrateIndex::new(url::Url::parse("https://github.com/rust-lang/crates.io-index.git").unwrap(), path, Default::default());

        // index.clone().unwrap();
    }

    #[test]
    fn test_downloads() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/tests/fixtures/crates.io-index");

        let index = super::CrateIndex::new(url::Url::parse("https://github.com/rust-lang/crates.io-index.git").unwrap(), path, Default::default());

        let mut crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crates.push("data/tests/fixtures/crates");

        // index.downloads(crates).unwrap();
    }
}
