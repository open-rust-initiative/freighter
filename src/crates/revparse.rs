///
///
/// ### References Codes
///
/// - [git2-rs](https://github.com/rust-lang/git2-rs)'s clone (example)[https://github.com/rust-lang/git2-rs/blob/master/examples/rev-parse.rs].
///
use git2::Repository;

use crate::errors::{FreightResult, FreighterError};

use super::index::CrateIndex;

pub fn run(index: &CrateIndex) -> FreightResult {
    println!("git reverse...");
    let path = index.path.to_str().map(|s| &s[..]).unwrap_or(".");
    let repo = Repository::open(path)?;

    let revspec = repo.revparse(&String::from("HEAD"))?;

    if revspec.mode().contains(git2::RevparseMode::SINGLE) {
        println!("{}", revspec.from().unwrap().id());
    } else if revspec.mode().contains(git2::RevparseMode::RANGE) {
        let to = revspec.to().unwrap();
        let from = revspec.from().unwrap();
        println!("{}", to.id());

        if revspec.mode().contains(git2::RevparseMode::MERGE_BASE) {
            let base = repo.merge_base(from.id(), to.id())?;
            println!("{}", base);
        }

        println!("^{}", from.id());
    } else {
        let err = FreighterError::new(
            anyhow::anyhow!("invalid results from revparse"),
            1,
        );
        return Err(err);
    }
    Ok(())
}
