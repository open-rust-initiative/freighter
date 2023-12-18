//! **server** subcommand focus on start a git proxy server and file server to fetch from local or upstream. The core
//! function implemented in the `src/server/server`.
//!
//!
//! **server** subcommand provide major functions include:
//!
//!   Arguments:
//!   - __ip__: start server with ip address
//!   - __port__: start server with port
//!
//! # handle crates file requests
//!   - crates file is in the format of "/crates/crates-name/0.1.0/download"
//!
//! # handle rustup-init file requests
//!   - rustup-init file is in the format of "/rustup/dist/aarch64-fuchsia"
//!   
//! # handle rust toolchain file requests
//!   - rust toolchain file is in the format of "/dist/2022-11-03/rust-1.65.0-aarch64-unknown-linux-gnu.tar.gz"
//!
//! # handle git client requests to crates.io-index
//!   - git client request is in the format of "/git/crates.io-index"

use std::net::IpAddr;
use std::path::PathBuf;

use clap::{arg, ArgMatches};

use crate::commands::command_prelude::*;
use crate::config::Config;
use crate::errors::FreightResult;
use crate::server::file_server::{self, FileServer};

pub fn cli() -> clap::Command {
    clap::Command::new("server")
        .arg(
            arg!(-i --"ip" <VALUE> "specify the ip address")
                .value_parser(value_parser!(IpAddr))
                .default_value("127.0.0.1"),
        )
        .arg(
            arg!(-p --"port" <VALUE> "specify the listening port")
                .value_parser(value_parser!(u16))
                .default_value("8000"),
        )
        .arg(
            arg!(-c --"cert-path" <VALUE> "Path to a TLS certificate file")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(-k --"key-path" <VALUE> "Path to a TLS key file")
                .value_parser(value_parser!(PathBuf)),
        )
        .about("Start git and file proxy server")
        .help_template(
            "\
Start the git proxy server and file http server.

USAGE:
    {usage}

OPTIONS:
{options}

EXAMPLES
1. Start server by port 8080

       freighter server -p 8080 

2. Start server with certificate

       freighter server -p 443 --cert-path /home/cert --key-path /home/key

\n",
        )
}

///
///
///
///
pub fn exec(config: &mut Config, args: &ArgMatches) -> FreightResult {
    let addr: IpAddr = args.get_one::<IpAddr>("ip").cloned().unwrap();
    let port: u16 = args.get_one::<u16>("port").cloned().unwrap();
    let cert_path: Option<PathBuf> = args.get_one::<PathBuf>("cert-path").cloned();
    let key_path: Option<PathBuf> = args.get_one::<PathBuf>("key-path").cloned();

    let file_server = &FileServer {
        cert_path,
        key_path,
        addr,
        port,
    };

    file_server::start(config, file_server);
    Ok(())
}
