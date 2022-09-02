//!
//!
//!
//!
//!

mod cli;
mod config;
mod errors;
mod commands;

fn main() {
    let mut config = config::Config::new();

    let result = cli::main(&mut config);

    match result {
        Err(e) => e.print(),
        Ok(()) => {}
    }
}
