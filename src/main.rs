//! [Freighter](https://github.com/open-rust-initiative/freighter) is an open source project to helping build the DevOps infrastructure for proxying the [crates.io](https://crates.io)
//! and provide simple registry functionality for local development.
//!
//!
//!
//!
mod cli;
mod cloud;
mod commands;
mod config;
mod handler;
mod download;
mod errors;
mod server;

///
/// Main entry point for the [Freighter](https://github.com/open-rust-initiative/freighter) application.
///
fn main() {
    let mut config = config::Config::new();

    let result = cli::main(&mut config);

    if let Err(e) = result {
        e.print()
    }
}
