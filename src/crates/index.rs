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
use rand::thread_rng;
use rand::seq::SliceRandom;
use sha2::{Digest, Sha256};

use std::collections::BTreeMap;
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Write};
use std::path::{Path, PathBuf};
use std::{fs, thread};
use std::time::Duration;
use std::env;
use std::str;

use crate::errors::FreightResult;

/// `CrateIndex` is a wrapper `Git Repository` that crates-io index.
///
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateIndex {
    pub url: Url,
    pub path: PathBuf,
}

/// State contains the progress when download crates file
///
///
pub struct State {
    pub progress: Option<Progress<'static>>,
    pub total: usize,
    pub current: usize,
    pub path: Option<PathBuf>,
    pub newline: bool,
}
/// SyncOptions preserve the sync subcommand config 
pub struct SyncOptions {
    /// Whether to hide processbar when start sync.
    pub no_processbar: bool
}

impl CrateIndex {
    /// default crate registry
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

/// Crate preserve the crate file info 
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

/// Dependencies maintain relationships between crate
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

/// DependencyKind represents which stage the current cependency is
///
///
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

/// CrateIndex impl provide several functions to for sync steps: like clone, pull, download 
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

    /// Check the destination path is a git repository and pull
    pub fn pull(&self, opts: &SyncOptions) -> FreightResult {
        let path = &self.path.to_str().map(|s| &s[..]).unwrap_or(".");
        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(e) => panic!("Target path is not a git repository: {}", e),
        };
        
        if cratesio_index_check(&repo) {
            // use default branch master
            let remote_branch = &String::from("master");
            // use default name origin
            let remote_name = &String::from("origin");
            let mut remote = repo.find_remote(remote_name).unwrap();
            let fetch_commit = do_fetch(&repo, &[remote_branch], &mut remote, opts)?;
            do_merge(&repo, &remote_branch, fetch_commit)
        } else {
            panic!("Target path is not a crates index: {}", &self.path.to_str().unwrap());
        }
    }

    /// Clone the `CrateIndex` to a local directory.
    ///
    ///
    pub fn clone(&self, opts: &mut SyncOptions) -> FreightResult {
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
            if !opts.no_processbar {
                print(&mut *state);
            }
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            if !opts.no_processbar {
                print(&mut *state);
            }
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

                        let url = format!("https://static.crates.io/crates/{}/{}-{}.crate", &c.name, &c.name, &c.vers);
                        let folder = path.join(&c.name);
                        let file = folder.join(format!("{}-{}.crate", &c.name, &c.vers));

                        if folder.exists() == false {
                            fs::create_dir_all(&folder).unwrap();
                        }

                        urls.push((url, file.to_str().unwrap().to_string(), c.cksum));
                    }
                }
            });

        let mut rng = thread_rng();
        urls.shuffle(&mut rng);

        let mut i = 0;
        for c in urls {
            let (url, file, cksum) = c;

            // https://github.com/RustScan/RustScan/wiki/Thread-main-paniced-at-too-many-open-files
            // 10,20,40,80,120,160,320
            if i % 10 == 0 {
                let mut rng = rand::thread_rng();
                thread::sleep(Duration::from_secs(rng.gen_range(1..5)));
            }

            let handle = thread::spawn(move || {
                let p = Path::new(&file);

                if p.is_file() == true && p.exists() == true {
                    let mut hasher = Sha256::new();
                    let mut f = File::open(p).unwrap();
                    io::copy(&mut f, &mut hasher).unwrap();
                    let result = hasher.finalize();
                    let hex = format!("{:x}", result);

                    if hex == cksum {
                        println!("###[ALREADY] \t{:?}", f);
                    } else {
                        let p = Path::new(&file);

                        println!("!!![REMOVE] \t\t {:?} !", f);
                        fs::remove_file(p).unwrap();

                        let mut resp = reqwest::blocking::get(url).unwrap();
                        let mut out = File::create(p).unwrap();
                        io::copy(&mut resp, &mut out).unwrap();

                        println!("!!![REMOVED DOWNLOAD] \t\t {:?}", out);
                    }
                } else {
                    let mut resp = reqwest::blocking::get(url).unwrap();
                    let mut out = File::create(file).unwrap();
                    io::copy(&mut resp, &mut out).unwrap();

                    println!("&&&[NEW] \t\t {:?}", out);
                }

            });
            handle.join().unwrap();

            i += 1;
        }

        Ok(())
    }
}

/// Check whether the directory is hidden
fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with("."))
        .unwrap_or(false)
}

/// Print processbar while clone data from git
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

/// If destination path is not empty, run pull instead of clone
pub fn run(index: CrateIndex, opts: &mut SyncOptions) -> FreightResult {
    if opts.no_processbar {
        println!("no-progressbar has been set to true, it will not be displayed!");
    }
    if Path::new(index.path.as_path()).exists() {
        index.pull(opts)?;
    } else {
        index.clone(opts)?;
    }
    Ok(())
}

/// Check the destination path is a crates-io index
pub fn cratesio_index_check(repo: &Repository) -> bool {
    let remote_name = &String::from("origin");
    let remote = repo.find_remote(remote_name).unwrap();
    let url = remote.url().unwrap(); 
    println!("current remote registry is: {}", url);
    if CrateIndex::CRATE_REGISTRY.contains(&url) {
        true
    } else {
        panic!("Traget url is not a crates index: {}", url)
    }
}

/// fetch the remote commit and show callback progress
fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
    opts:& SyncOptions,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::new();

    // Print out our transfer progress.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        io::stdout().flush().unwrap();
        true
    });

    let mut fo = git2::FetchOptions::new();

    if !opts.no_processbar {
        fo.remote_callbacks(cb);
    }

    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    println!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        println!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        println!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
}

/// Set repo head to the newest remote commit
fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}


/// Add a merge commit and set working tree to match head
fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        println!("Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

/// Do a merge analysis to determine wether it should fast_forward or merge
fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> FreightResult {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        println!("Nothing to do...");
    }
    Ok(())
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
