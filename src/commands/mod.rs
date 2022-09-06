//!
//!
//!
//!
//!

use clap::ArgMatches;

use crate::cli::App;
use crate::config::Config;
use crate::errors::FreightResult;

pub(crate) mod sync;

pub fn builtin() -> Vec<App> {
    vec![
        sync::cli(),
    ]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&mut Config, &ArgMatches) -> FreightResult> {
    let f = match cmd {
        "sync" => sync::exec,
        _ => return None,
    };

    Some(f)
}