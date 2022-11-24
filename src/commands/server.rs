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
//!   - rustup-init file is in the format of "/rustup/dist/aarch64-fuschia"
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
use crate::server::file_server::{self, parse_ipaddr};

pub fn cli() -> clap::Command {
    clap::Command::new("server")
        .arg(arg!(-i --"ip" <VALUE> "specify the ip address").value_parser(value_parser!(IpAddr)))
        .arg(
            arg!(-p --"port" <VALUE> "specify the listening port").value_parser(value_parser!(u16)),
        )
        .arg(
            arg!(-d --"directory" <VALUE> "specify the file directory")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg_required_else_help(true)
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


\n",
        )
}

///
///
///
///
pub fn exec(config: &mut Config, args: &ArgMatches) -> FreightResult {

    let listen: Option<IpAddr> = args.get_one::<IpAddr>("ip").cloned();
    let port: Option<u16> = args.get_one::<u16>("port").cloned();

    let socket_addr = parse_ipaddr(listen, port);
    file_server::start(config, socket_addr);
    Ok(())
}
