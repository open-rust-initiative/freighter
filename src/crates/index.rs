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
use chrono::Utc;
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
    DiffFormat, DiffLine, DiffOptions, ErrorCode, FetchOptions, Object, ObjectType, Oid, Progress,
    RemoteCallbacks, Repository,
};

use serde::{Deserialize, Serialize};
use url::Url;
use walkdir::{DirEntry, WalkDir};

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

use crate::config::Config;
use crate::download::{
    download_crates_with_log, sync_folder
};
use crate::errors::FreightResult;

/// `CrateIndex` is a wrapper `Git Repository` that crates-io index.
///
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrateIndex {
    pub url: Url,
    /// index path
    pub path: PathBuf,
    //download crates path
    pub crates_path: PathBuf,
    pub log_path: PathBuf,
    pub rustup_path: PathBuf,
    pub dist_path: PathBuf,
    pub thread_count: usize,
    // upload file after download
    pub upload: bool,
    pub work_dir: PathBuf,
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
#[derive(Default)]
pub struct SyncOptions {
    /// Whether to hide progressbar when start sync.
    pub no_progressbar: bool,
    /// start traverse all directories
    pub init: bool,
}

impl CrateIndex {
    /// default crate registry
    const CRATE_REGISTRY: [&str; 3] = ["https://github.com/rust-lang/crates.io-index.git", "", ""];
    const RECORD_NAME: &str = "record.cache";
    const ERROR_CRATES: &str = "error-crates.cache";
    // use default branch master
    const REMOTE_BRANCH: &str = "master";
    // use default name origin
    const REMOTE_NAME: &str = "origin";
}

impl Default for CrateIndex {
    fn default() -> CrateIndex {
        let home_path = dirs::home_dir().unwrap();
        CrateIndex {
            url: Url::parse(CrateIndex::CRATE_REGISTRY[0]).unwrap(),
            path: home_path.join("freighter/crates.io-index"),
            crates_path: home_path.join("freighter/crates"),
            log_path: home_path.join("freighter/log"),
            rustup_path: home_path.join("freighter/rustup"),
            dist_path: home_path.join("freighter/dist"),
            thread_count: 16,
            upload: false,
            work_dir: home_path,
        }
    }
}

/// Crate preserve the crates info parse from registry json file
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

/// DependencyKind represents which stage the current dependency is
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
    /// Create a new `CrateIndex` from a `Work dir`.
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            path: work_dir.join("freighter/crates.io-index"),
            crates_path: work_dir.join("freighter/crates"),
            log_path: work_dir.join("freighter/log"),
            rustup_path: work_dir.join("freighter/rustup"),
            dist_path: work_dir.join("freighter/dist"),
            work_dir,
            ..Default::default()
        }
    }

    /// Get the `path` of this `CrateIndex`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check the destination path is a git repository and pull
    pub fn git_pull(&self, opts: &SyncOptions) -> FreightResult {
        let repo = get_repo(self.path.clone());

        if crates_io_index_check(&repo) {
            let mut remote = repo.find_remote(CrateIndex::REMOTE_NAME).unwrap();
            let object = repo.revparse_single(CrateIndex::REMOTE_BRANCH)?;
            let commit = object.peel_to_commit()?;
            let fetch_commit = do_fetch(&repo, &[CrateIndex::REMOTE_BRANCH], &mut remote, opts)?;

            self.generate_commit_record(&commit.id(), &fetch_commit.id());
            println!(
                "commit id：{}， remote id :{}",
                commit.id(),
                &fetch_commit.id()
            );
            do_merge(&repo, CrateIndex::REMOTE_BRANCH, fetch_commit)
        } else {
            panic!(
                "Target path is not a crates index: {}",
                &self.path.to_str().unwrap()
            );
        }
    }

    /// Clone the `CrateIndex` to a local directory.
    ///
    ///
    pub fn git_clone(&self, opts: &mut SyncOptions) -> FreightResult {
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
            if !opts.no_progressbar {
                print(&mut state);
            }
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();
            state.path = path.map(|p| p.to_path_buf());
            state.current = cur;
            state.total = total;
            if !opts.no_progressbar {
                print(&mut state);
            }
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        let repo = RepoBuilder::new()
            .fetch_options(fo)
            .with_checkout(co)
            .clone(self.url.as_ref(), self.path.as_path())?;

        let object = repo.revparse_single(CrateIndex::REMOTE_BRANCH)?;
        let commit = object.peel_to_commit()?;
        // first commit of crates.io-index
        self.generate_commit_record(
            &Oid::from_str("83ef4b3aa2e01d0cba0d267a68780aec797dd5f1").unwrap(),
            &commit.id(),
        );
        Ok(())
    }

    /// save commit record in record.cache, it will write from first commit to current commit if command is git clone
    pub fn generate_commit_record(&self, start_commit_id: &Oid, end_commit_id: &Oid) {
        let now = Utc::now();
        let mut file_name = now.date().to_string();
        file_name.push('-');
        file_name.push_str(CrateIndex::RECORD_NAME);
        let file_name = &self.log_path.join(file_name);
        let mut f = match OpenOptions::new().write(true).append(true).open(file_name) {
            Ok(f) => f,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    fs::create_dir_all(&self.log_path).unwrap();
                    File::create(file_name).unwrap()
                }
                other_error => panic!("something wrong: {}", other_error),
            },
        };
        // save record commit id only id does not matches
        if start_commit_id != end_commit_id {
            writeln!(
                f,
                "{},{},{}",
                start_commit_id,
                end_commit_id,
                now.timestamp()
            )
            .unwrap();
        }
    }

    /// Check whether the directory is hidden
    pub fn is_not_hidden(&self, entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| entry.depth() == 0 || !s.starts_with('.'))
            .unwrap_or(false)
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
    pub fn full_downloads(&self, config: &Config) -> FreightResult {
        let pool = ThreadPool::new(self.thread_count);
        let err_record = open_file_with_mutex(&self.log_path);

        WalkDir::new(self.path())
            .into_iter()
            .filter_entry(|e| self.is_not_hidden(e))
            .filter_map(|v| v.ok())
            .for_each(|x| {
                if x.file_type().is_file() && x.path().extension().unwrap_or_default() != "json" {
                    let input = File::open(x.path()).unwrap();
                    let buffered = BufReader::new(input);

                    for line in buffered.lines() {
                        let line = line.unwrap();
                        let c: Crate = serde_json::from_str(&line).unwrap();
                        let index = self.clone();
                        let config = config.clone();
                        let err_record = Arc::clone(&err_record);
                        pool.execute(move || {
                            download_crates_with_log(
                                index,
                                config,
                                c,
                                err_record,
                            );
                        });
                    }
                }
            });

        pool.join();

        println!("sync ends with {} task failed", pool.panic_count());
        Ok(())
    }
}

/// Print progressbar while clone data from git
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
pub fn pull(index: CrateIndex, opts: &mut SyncOptions) -> FreightResult {
    if opts.no_progressbar {
        println!("no-progressbar has been set to true, it will not be displayed!");
    }
    let index_dir = Path::new(index.path.as_path());
    // try to remove index dir if it's empty
    if index_dir.exists() {
        if !index_dir
            .read_dir()
            .unwrap()
            .filter_map(|x| x.ok())
            .any(|e| !e.file_name().to_str().unwrap().contains("git"))
        {
            println!(
                "It seems last task has been broken and {} is empty, 
            freighter had to removed this index, and then run init again",
                index_dir.display()
            );
            match fs::remove_dir_all(index_dir) {
                Ok(_) => index.git_clone(opts).unwrap(),
                Err(e) => panic!("Remove index failed, try to delete it manualy: {}", e),
            };
        } else {
            index.git_pull(opts).unwrap();
        }
    } else {
        index.git_clone(opts).unwrap();
    }
    Ok(())
}

/// full download and Incremental download from registry
pub fn download(index: CrateIndex, config: &Config, opts: &mut SyncOptions) -> FreightResult {
    if opts.init {
        index.full_downloads(config).unwrap();
    } else {
        let it = WalkDir::new(&index.log_path)
            .into_iter()
            .filter_entry(|e| {
                e.file_name()
                    .to_str()
                    .unwrap()
                    .contains(&Utc::now().date().to_string())
                    || e.file_type().is_dir()
            })
            .filter_map(|v| v.ok());
        let mut input = match it.last() {
            Some(dir) => {
                if dir.file_type().is_file() {
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(dir.path())
                        .unwrap()
                } else {
                    panic!("Cannot get record file, run freighter sync pull before download")
                }
            }
            None => panic!("Did you forget to run freighter sync pull before download?"),
        };
        let buffered = BufReader::new(&mut input);
        println!("crates.io-index modified:");
        let err_record = open_file_with_mutex(&index.log_path);
        // get last line of record file
        let mut lines: Vec<String> = buffered.lines().map(|line| line.unwrap()).collect();
        lines.reverse();
        if let Some(line) = lines.first() {
            let vec: Vec<&str> = line.split(',').collect();
            println!("{:?}", line);
            git2_diff(&index, config, vec[0], vec[1], err_record).unwrap();
        }
    }

    Ok(())
}

pub fn upload_to_s3(index: CrateIndex, bucket_name: &str) -> FreightResult {
    let sync_paths = [&index.crates_path, &index.rustup_path, &index.dist_path];
    for path in sync_paths {
        sync_folder(path.to_str().unwrap(), bucket_name).unwrap();
    }
    Ok(())
}

/// get repo from path
pub fn get_repo(path: PathBuf) -> Repository {
    let path = path.to_str().unwrap_or(".");
    match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => match e.code() {
            ErrorCode::NotFound => {
                panic!(
                    "index path: {} not found, please execute freighter sync pull first",
                    &path
                );
            }
            _other_error => panic!("Target path is not a git repository: {}", e),
        },
    }
}

pub fn git2_diff(
    index: &CrateIndex,
    config: &Config,
    from_oid: &str,
    to_oid: &str,
    file: Arc<Mutex<File>>,
) -> Result<(), anyhow::Error> {
    let repo = get_repo(index.path.clone());
    let t1 = tree_to_treeish(&repo, from_oid)?;
    let t2 = tree_to_treeish(&repo, to_oid)?;
    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(
        t1.unwrap().as_tree(),
        t2.unwrap().as_tree(),
        Some(&mut opts),
    )?;
    let pool = ThreadPool::new(index.thread_count);

    diff.print(DiffFormat::NameOnly, |_d, _h, l| {
        handle_diff_line(l, index, config, &pool, &file)
    })?;

    pool.join();

    Ok(())
}

/// Traversing directories in diff lines
fn handle_diff_line(
    line: DiffLine,
    index: &CrateIndex,
    config: &Config,
    pool: &ThreadPool,
    err_record: &Arc<Mutex<File>>,
) -> bool {
    let path_suffix = str::from_utf8(line.content())
        .unwrap()
        .strip_suffix('\n')
        .unwrap();
    if path_suffix.eq("config.json") {
        return true;
    }
    let crate_path = index.path.join(path_suffix);
    match File::open(&crate_path) {
        Ok(f) => {
            let buffered = BufReader::new(f);

            for line in buffered.lines() {
                let line = line.unwrap();
                let c: Crate = serde_json::from_str(&line).unwrap();
                let err_record = Arc::clone(err_record);
                let index = index.to_owned();
                let config = config.to_owned();
                pool.execute(move || {
                    download_crates_with_log(index, config, c, err_record);
                });
            }
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                println!(
                    "This file might have been removed from crates.io:{}",
                    &crate_path.display()
                );
            }
            other_error => panic!(
                "something wrong while open the crates file: {}",
                other_error
            ),
        },
    };

    true
}

/// open error record file with Mutex
pub fn open_file_with_mutex(log_path: &Path) -> Arc<Mutex<File>> {
    let file_name = log_path.join(CrateIndex::ERROR_CRATES);
    let err_record = match OpenOptions::new().write(true).append(true).open(&file_name) {
        Ok(f) => Arc::new(Mutex::new(f)),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Arc::new(Mutex::new(File::create(&file_name).unwrap())),
            other_error => panic!("something wrong: {}", other_error),
        },
    };
    err_record
}

/// ### References Codes
///
/// - [git2-rs](https://github.com/rust-lang/git2-rs)'s clone (example)[https://github.com/rust-lang/git2-rs/blob/master/examples/diff.rs].
fn tree_to_treeish<'a>(
    repo: &'a Repository,
    arg: &str,
) -> Result<Option<Object<'a>>, anyhow::Error> {
    let obj = repo.revparse_single(arg)?;
    let tree = obj.peel(ObjectType::Tree)?;
    Ok(Some(tree))
}

/// Check the destination path is a crates-io index
pub fn crates_io_index_check(repo: &Repository) -> bool {
    let remote_name = &String::from("origin");
    let remote = repo.find_remote(remote_name).unwrap();
    let url = remote.url().unwrap();
    println!("current remote registry is: {}", url);
    if CrateIndex::CRATE_REGISTRY.contains(&url) {
        true
    } else {
        panic!("Target url is not a crates index: {}", url)
    }
}

/// fetch the remote commit and show callback progress
fn do_fetch<'a>(
    repo: &'a Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
    opts: &SyncOptions,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = RemoteCallbacks::new();

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

    let mut fo = FetchOptions::new();

    if !opts.no_progressbar {
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
    repo.reference_to_annotated_commit(&fetch_head)
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
        CheckoutBuilder::default()
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
        println!("Merge conflicts detected...");
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
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    Ok(())
}

/// Do a merge analysis to determine whether it should fast_forward or merge
fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> FreightResult {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appropriate merge
    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");
        // do a fast forward
        let ref_name = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&ref_name) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &ref_name,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&ref_name)?;
                repo.checkout_head(Some(
                    CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
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
        path.push("data/tests/fixtures/");

        let mut index = super::CrateIndex::new(path);
        index.url = url::Url::parse("https://github.com/rust-lang/crates.io-index.git").unwrap();
    }

    #[test]
    fn test_downloads() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/tests/fixtures/");

        let mut index = super::CrateIndex::new(path);
        index.url = url::Url::parse("https://github.com/rust-lang/crates.io-index.git").unwrap();
    }
}
